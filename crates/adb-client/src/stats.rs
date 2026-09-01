use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};

use crate::adb::run_adb_for_serial;
use crate::error::AdbError;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Memory statistics from `/proc/meminfo` (values in kB).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStats {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
}

impl MemoryStats {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }

    pub fn used_fraction(&self) -> f32 {
        if self.total_kb == 0 {
            0.0
        } else {
            self.used_kb() as f32 / self.total_kb as f32
        }
    }
}

/// Disk usage from `df -h`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskStats {
    pub filesystem: String,
    pub size: String,
    pub used: String,
    pub available: String,
    pub use_percent: String,
    pub mount_point: String,
}

/// Snapshot of device memory and disk usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemStats {
    pub memory: MemoryStats,
    pub disks: Vec<DiskStats>,
    pub timestamp: SystemTime,
}

/// Background poller for memory and disk stats.
pub struct StatsPoller {
    stop_tx: Sender<()>,
    join_handle: Option<JoinHandle<()>>,
}

impl StatsPoller {
    /// Polls device stats every two seconds.
    pub fn spawn(serial: &str) -> Result<(Receiver<SystemStats>, Self), AdbError> {
        Self::spawn_with_interval(serial, DEFAULT_POLL_INTERVAL)
    }

    pub fn spawn_with_interval(
        serial: &str,
        interval: Duration,
    ) -> Result<(Receiver<SystemStats>, Self), AdbError> {
        let (stats_tx, stats_rx) = crossbeam_channel::unbounded();
        let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
        let serial = serial.to_string();

        let join_handle = thread::spawn(move || {
            while stop_rx.try_recv().is_err() {
                match fetch_system_stats(&serial) {
                    Ok(stats) => {
                        if stats_tx.send(stats).is_err() {
                            break;
                        }
                    }
                    Err(_) => {}
                }

                sleep_until_stop(&stop_rx, interval);
            }
        });

        Ok((
            stats_rx,
            StatsPoller {
                stop_tx,
                join_handle: Some(join_handle),
            },
        ))
    }

    pub fn stop(mut self) {
        let _ = self.stop_tx.send(());
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StatsPoller {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

pub(crate) fn fetch_system_stats(serial: &str) -> Result<SystemStats, AdbError> {
    let meminfo = run_adb_for_serial(serial, &["shell", "cat", "/proc/meminfo"])?;
    let df = run_adb_for_serial(serial, &["shell", "df", "-h"])?;

    let memory = parse_meminfo(&String::from_utf8_lossy(&meminfo.stdout))?;
    let disks = parse_df(&String::from_utf8_lossy(&df.stdout));

    Ok(SystemStats {
        memory,
        disks,
        timestamp: SystemTime::now(),
    })
}

fn parse_meminfo(text: &str) -> Result<MemoryStats, AdbError> {
    let mut total_kb = None;
    let mut free_kb = None;
    let mut available_kb = None;
    let mut buffers_kb = None;
    let mut cached_kb = None;

    for line in text.lines() {
        let (key, value) = match line.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        let kb = parse_kb_value(value)?;
        match key.trim() {
            "MemTotal" => total_kb = Some(kb),
            "MemFree" => free_kb = Some(kb),
            "MemAvailable" => available_kb = Some(kb),
            "Buffers" => buffers_kb = Some(kb),
            "Cached" => cached_kb = Some(kb),
            _ => {}
        }
    }

    Ok(MemoryStats {
        total_kb: total_kb.ok_or_else(|| AdbError::ParseFailed("missing MemTotal".into()))?,
        free_kb: free_kb.unwrap_or(0),
        available_kb: available_kb.unwrap_or(0),
        buffers_kb: buffers_kb.unwrap_or(0),
        cached_kb: cached_kb.unwrap_or(0),
    })
}

fn parse_kb_value(raw: &str) -> Result<u64, AdbError> {
    let kb = raw
        .trim()
        .split_whitespace()
        .next()
        .ok_or_else(|| AdbError::ParseFailed(format!("invalid meminfo value: {raw}")))?;

    kb.parse()
        .map_err(|_| AdbError::ParseFailed(format!("invalid meminfo kB value: {raw}")))
}

fn parse_df(text: &str) -> Vec<DiskStats> {
    text.lines()
        .skip(1)
        .filter_map(parse_df_line)
        .collect()
}

fn parse_df_line(line: &str) -> Option<DiskStats> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    Some(DiskStats {
        filesystem: parts[0].to_string(),
        size: parts[1].to_string(),
        used: parts[2].to_string(),
        available: parts[3].to_string(),
        use_percent: parts[4].to_string(),
        mount_point: parts[5..].join(" "),
    })
}

fn sleep_until_stop(stop_rx: &Receiver<()>, duration: Duration) {
    let step = Duration::from_millis(100);
    let mut elapsed = Duration::ZERO;

    while elapsed < duration {
        if stop_rx.try_recv().is_ok() {
            return;
        }
        thread::sleep(step);
        elapsed += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meminfo_sample() {
        let sample = r#"MemTotal:        8025332 kB
MemFree:          123456 kB
MemAvailable:    3456789 kB
Buffers:          111111 kB
Cached:           222222 kB
"#;

        let memory = parse_meminfo(sample).unwrap();
        assert_eq!(memory.total_kb, 8_025_332);
        assert_eq!(memory.free_kb, 123_456);
        assert_eq!(memory.available_kb, 3_456_789);
        assert_eq!(memory.buffers_kb, 111_111);
        assert_eq!(memory.cached_kb, 222_222);
        assert_eq!(memory.used_kb(), 8_025_332 - 3_456_789);
    }

    #[test]
    fn parse_df_sample() {
        let sample = r#"Filesystem       Size Used Avail Use% Mounted on
/dev/block/dm-5   55G  12G   42G  23% /data
tmpfs            3.8G    0  3.8G   0% /dev
"#;

        let disks = parse_df(sample);
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].filesystem, "/dev/block/dm-5");
        assert_eq!(disks[0].mount_point, "/data");
        assert_eq!(disks[0].use_percent, "23%");
    }
}
