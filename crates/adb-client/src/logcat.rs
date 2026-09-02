use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::LazyLock;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use regex::Regex;

use crate::error::AdbError;

static LOGCAT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEF])\s+([^:]*):\s?(.*)$",
    )
    .expect("valid logcat regex")
});

/// A single line from `adb logcat -v threadtime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub timestamp: String,
    pub pid: u32,
    pub tid: u32,
    pub level: char,
    pub tag: String,
    pub message: String,
}

impl LogEntry {
    pub fn format_line(&self) -> String {
        format!(
            "{} {:>5} {:>5} {} {}: {}",
            self.timestamp, self.pid, self.tid, self.level, self.tag, self.message
        )
    }

    /// Returns true for Error (`E`) and Fatal (`F`) log levels.
    pub fn is_error_level(&self) -> bool {
        matches!(self.level, 'E' | 'F')
    }

    /// Diagnostic message from adb/logcat (stderr, spawn failures, etc.).
    pub fn adb_diagnostic(message: impl Into<String>) -> Self {
        LogEntry {
            timestamp: String::new(),
            pid: 0,
            tid: 0,
            level: 'E',
            tag: "adb".to_string(),
            message: message.into(),
        }
    }

    fn raw(message: String) -> Self {
        Self::adb_diagnostic(message)
    }
}

/// Streaming handle for an `adb logcat` subprocess.
pub struct LogcatStream {
    stop_tx: Sender<()>,
    join_handle: Option<JoinHandle<()>>,
}

impl LogcatStream {
    /// Spawns `adb -s <serial> logcat -v threadtime` and returns a receiver of parsed entries.
    pub fn spawn(serial: &str) -> Result<(Receiver<LogEntry>, Self), AdbError> {
        Self::spawn_with_filters(serial, Vec::new())
    }

    /// Spawns a separate logcat process filtered to Error and Fatal (`*:E`).
    pub fn spawn_errors(serial: &str) -> Result<(Receiver<LogEntry>, Self), AdbError> {
        Self::spawn_with_filters(serial, vec!["*:E".to_string()])
    }

    fn spawn_with_filters(
        serial: &str,
        filters: Vec<String>,
    ) -> Result<(Receiver<LogEntry>, Self), AdbError> {
        let (entry_tx, entry_rx) = crossbeam_channel::unbounded();
        let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
        let serial = serial.to_string();

        let join_handle = thread::spawn(move || {
            let child = match spawn_logcat_child(&serial, &filters) {
                Ok(child) => child,
                Err(err) => {
                    let _ = entry_tx.send(LogEntry::raw(format!("logcat error: {err}")));
                    return;
                }
            };

            let child = Arc::new(std::sync::Mutex::new(child));
            stream_logcat(child.clone(), entry_tx, stop_rx);
            kill_child(&child);
        });

        Ok((
            entry_rx,
            LogcatStream {
                stop_tx,
                join_handle: Some(join_handle),
            },
        ))
    }

    /// Stops the background logcat stream and waits for the reader thread to exit.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for LogcatStream {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_logcat_child(serial: &str, filters: &[String]) -> Result<Child, AdbError> {
    Command::new("adb")
        .args(["-s", serial, "logcat", "-v", "threadtime"])
        .args(filters)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AdbError::NotFound
            } else {
                AdbError::Io(err)
            }
        })
}

fn stream_logcat(
    child: Arc<std::sync::Mutex<Child>>,
    entry_tx: Sender<LogEntry>,
    stop_rx: Receiver<()>,
) {
    let stderr_tx = entry_tx.clone();
    let stderr_child = child.clone();
    thread::spawn(move || read_logcat_stderr(stderr_child, stderr_tx));

    let stdout = {
        let mut guard = match child.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        match guard.stdout.take() {
            Some(stdout) => stdout,
            None => return,
        }
    };

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        if stop_rx.try_recv().is_ok() {
            break;
        }

        let line = match line {
            Ok(line) => line,
            Err(err) => {
                let _ = entry_tx.send(LogEntry::raw(format!("logcat read error: {err}")));
                break;
            }
        };

        if let Some(entry) = parse_logcat_line(&line) {
            if entry_tx.send(entry).is_err() {
                break;
            }
        }

        if stop_rx.try_recv().is_ok() {
            break;
        }
    }

    report_logcat_exit(&child, &entry_tx);
}

fn read_logcat_stderr(child: Arc<std::sync::Mutex<Child>>, entry_tx: Sender<LogEntry>) {
    let stderr = {
        let mut guard = match child.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        guard.stderr.take()
    };

    let Some(stderr) = stderr else {
        return;
    };

    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        match line {
            Ok(line) if !line.trim().is_empty() => {
                let _ = entry_tx.send(LogEntry::adb_diagnostic(line));
            }
            Ok(_) => {}
            Err(err) => {
                let _ = entry_tx.send(LogEntry::adb_diagnostic(format!("logcat stderr read error: {err}")));
                break;
            }
        }
    }
}

fn report_logcat_exit(child: &Arc<std::sync::Mutex<Child>>, entry_tx: &Sender<LogEntry>) {
    if let Ok(mut guard) = child.lock() {
        if let Ok(status) = guard.wait() {
            if !status.success() {
                let _ = entry_tx.send(LogEntry::adb_diagnostic(format!("logcat exited: {status}")));
            }
        }
    }
}

fn kill_child(child: &Arc<std::sync::Mutex<Child>>) {
    if let Ok(mut guard) = child.lock() {
        let _ = guard.kill();
        for _ in 0..10 {
            if guard.try_wait().ok().flatten().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = guard.wait();
    }
}

fn parse_logcat_line(line: &str) -> Option<LogEntry> {
    let captures = LOGCAT_LINE.captures(line)?;
    let pid = captures.get(2)?.as_str().parse().ok()?;
    let tid = captures.get(3)?.as_str().parse().ok()?;

    Some(LogEntry {
        timestamp: captures.get(1)?.as_str().to_string(),
        pid,
        tid,
        level: captures.get(4)?.as_str().chars().next()?,
        tag: captures.get(5)?.as_str().to_string(),
        message: captures.get(6).map(|m| m.as_str().to_string()).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adb_diagnostic_is_error_level() {
        let entry = LogEntry::adb_diagnostic("device offline");
        assert!(entry.is_error_level());
        assert_eq!(entry.tag, "adb");
    }

    #[test]
    fn parse_standard_logcat_line() {
        let entry = parse_logcat_line("03-15 10:23:45.123  1234  5678 I MyTag: Hello world").unwrap();

        assert_eq!(entry.timestamp, "03-15 10:23:45.123");
        assert_eq!(entry.pid, 1234);
        assert_eq!(entry.tid, 5678);
        assert_eq!(entry.level, 'I');
        assert_eq!(entry.tag, "MyTag");
        assert_eq!(entry.message, "Hello world");
    }

    #[test]
    fn parse_error_logcat_line() {
        let entry = parse_logcat_line(
            "09-01 17:00:01.456  9999  9999 E AndroidRuntime: FATAL EXCEPTION",
        )
        .unwrap();

        assert_eq!(entry.level, 'E');
        assert_eq!(entry.tag, "AndroidRuntime");
        assert_eq!(entry.message, "FATAL EXCEPTION");
    }

    #[test]
    fn parse_line_without_message() {
        let entry = parse_logcat_line("09-01 17:00:01.456  100  100 W Tag:").unwrap();

        assert_eq!(entry.level, 'W');
        assert_eq!(entry.tag, "Tag");
        assert_eq!(entry.message, "");
    }

    #[test]
    fn skip_unrecognized_line() {
        assert!(parse_logcat_line("--------- beginning of main").is_none());
    }

    #[test]
    fn error_level_filter() {
        let error = parse_logcat_line("09-01 17:00:01.456  1  1 E Tag: boom").unwrap();
        let fatal = parse_logcat_line("09-01 17:00:01.456  1  1 F Tag: boom").unwrap();
        let warning = parse_logcat_line("09-01 17:00:01.456  1  1 W Tag: boom").unwrap();

        assert!(error.is_error_level());
        assert!(fatal.is_error_level());
        assert!(!warning.is_error_level());
    }
}
