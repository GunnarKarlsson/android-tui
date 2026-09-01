use std::io;
use std::process::{Command, Output};

use crate::error::AdbError;

/// Entry point for running `adb` commands.
pub struct Adb;

impl Adb {
    /// Verifies that `adb` is installed and runnable.
    pub fn check_available() -> Result<(), AdbError> {
        let output = run_adb(&["version"])?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            Err(AdbError::VersionCheckFailed(stderr))
        }
    }

    /// Lists devices attached to the host via `adb devices -l`.
    pub fn list_devices() -> Result<Vec<crate::device::DeviceInfo>, AdbError> {
        let output = run_adb(&["devices", "-l"])?;
        crate::device::devices_from_output(&output)
    }
}

pub(crate) fn run_adb(args: &[&str]) -> Result<Output, AdbError> {
    let output = Command::new("adb")
        .args(args)
        .output()
        .map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                AdbError::NotFound
            } else {
                AdbError::Io(err)
            }
        })?;

    if output.status.success() {
        Ok(output)
    } else {
        let command = format!("adb {}", args.join(" "));
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(AdbError::CommandFailed { command, stderr })
    }
}
