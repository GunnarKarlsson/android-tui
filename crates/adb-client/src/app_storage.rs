use std::sync::LazyLock;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use regex::Regex;

use crate::adb::run_adb_for_serial;
use crate::error::AdbError;

const INITIAL_DELAY: Duration = Duration::from_secs(3);
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

static STORAGE_BYTES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(.+?): (\d+) bytes").expect("valid storage bytes regex")
});

/// Per-app storage sizes from `cmd package get-package-storage-stats`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageStorage {
    pub package: String,
    pub total_bytes: u64,
}

/// Incremental update from the app storage poller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStorageUpdate {
    PackageList(Vec<String>),
    PackageStorage(PackageStorage),
    ScanComplete,
    Error(String),
}

/// Background poller that streams app storage sizes as they are measured.
pub struct AppStoragePoller {
    stop_tx: Sender<()>,
    join_handle: Option<JoinHandle<()>>,
}

impl AppStoragePoller {
    pub fn spawn(serial: &str) -> Result<(Receiver<AppStorageUpdate>, Self), AdbError> {
        let (update_tx, update_rx) = crossbeam_channel::unbounded();
        let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
        let serial = serial.to_string();

        let join_handle = thread::spawn(move || {
            sleep_until_stop(&stop_rx, INITIAL_DELAY);
            while stop_rx.try_recv().is_err() {
                if run_scan(&serial, &update_tx, &stop_rx).is_err() {
                    break;
                }
                sleep_until_stop(&stop_rx, REFRESH_INTERVAL);
            }
        });

        Ok((
            update_rx,
            AppStoragePoller {
                stop_tx,
                join_handle: Some(join_handle),
            },
        ))
    }

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

impl Drop for AppStoragePoller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_scan(serial: &str, update_tx: &Sender<AppStorageUpdate>, stop_rx: &Receiver<()>) -> Result<(), ()> {
    let packages = match fetch_package_list(serial) {
        Ok(packages) => packages,
        Err(err) => {
            if update_tx
                .send(AppStorageUpdate::Error(err.user_message()))
                .is_err()
            {
                return Err(());
            }
            return Ok(());
        }
    };

    if update_tx
        .send(AppStorageUpdate::PackageList(packages.clone()))
        .is_err()
    {
        return Err(());
    }

    for package in packages {
        if stop_rx.try_recv().is_ok() {
            return Err(());
        }

        let total_bytes = fetch_package_storage(serial, &package)
            .unwrap_or(0);
        if update_tx
            .send(AppStorageUpdate::PackageStorage(PackageStorage {
                package,
                total_bytes,
            }))
            .is_err()
        {
            return Err(());
        }
    }

    if update_tx.send(AppStorageUpdate::ScanComplete).is_err() {
        return Err(());
    }

    Ok(())
}

fn fetch_package_list(serial: &str) -> Result<Vec<String>, AdbError> {
    let output = run_adb_for_serial(serial, &["shell", "pm", "list", "packages"])?;
    let mut packages: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("package:").map(str::trim).map(str::to_string))
        .collect();
    packages.sort();
    Ok(packages)
}

fn fetch_package_storage(serial: &str, package: &str) -> Result<u64, AdbError> {
    let output = run_adb_for_serial(
        serial,
        &["shell", "cmd", "package", "get-package-storage-stats", package],
    )?;
    Ok(parse_storage_stats_total(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_storage_stats_total(text: &str) -> u64 {
    STORAGE_BYTES
        .captures_iter(text)
        .filter_map(|caps| caps.get(2)?.as_str().parse::<u64>().ok())
        .sum()
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
    fn parse_storage_stats_total_sums_bytes() {
        let sample = r#"code: 125628416 bytes (119.81 Mb)
data: 9515008 bytes (9.07 Mb)
cache: 184320 bytes (180.00 Kb)
apk: 70806309 bytes (67.53 Mb)
dexopt artifacts: 54725272 bytes (52.19 Mb)
"#;
        assert_eq!(
            parse_storage_stats_total(sample),
            125_628_416 + 9_515_008 + 184_320 + 70_806_309 + 54_725_272
        );
    }

    #[test]
    fn parse_package_list() {
        let sample = "package:com.android.chrome\npackage:com.google.android.gms\n";
        let list = sample
            .lines()
            .filter_map(|line| line.strip_prefix("package:").map(str::trim).map(str::to_string))
            .collect::<Vec<_>>();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], "com.android.chrome");
    }
}
