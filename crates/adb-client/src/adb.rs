use std::io;
use std::process::Command;

use crate::error::AdbError;

/// Entry point for running `adb` commands.
pub struct Adb;

impl Adb {
    /// Verifies that `adb` is installed and runnable.
    pub fn check_available() -> Result<(), AdbError> {
        let output = Command::new("adb")
            .arg("version")
            .output()
            .map_err(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    AdbError::NotFound
                } else {
                    AdbError::Io(err)
                }
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            Err(AdbError::VersionCheckFailed(stderr))
        }
    }
}
