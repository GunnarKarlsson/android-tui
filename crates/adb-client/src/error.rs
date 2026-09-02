use std::io;

#[derive(Debug, thiserror::Error)]
pub enum AdbError {
    #[error(
        "adb not found on PATH. Install Android SDK platform-tools and ensure `adb` is available."
    )]
    NotFound,

    #[error("failed to run adb: {0}")]
    Io(io::Error),

    #[error("adb version check failed: {0}")]
    VersionCheckFailed(String),

    #[error("adb command failed ({command}): {stderr}")]
    CommandFailed { command: String, stderr: String },

    #[error("failed to parse adb output: {0}")]
    ParseFailed(String),
}

impl AdbError {
    /// Returns a short, user-facing description of the error.
    pub fn user_message(&self) -> String {
        match self {
            AdbError::NotFound => {
                "adb not found on PATH. Install Android SDK platform-tools.".to_string()
            }
            AdbError::Io(err) => format!("Failed to run adb: {err}"),
            AdbError::VersionCheckFailed(stderr) => classify_adb_stderr(stderr),
            AdbError::CommandFailed { stderr, .. } => classify_adb_stderr(stderr),
            AdbError::ParseFailed(message) => format!("Failed to parse device output: {message}"),
        }
    }
}

fn classify_adb_stderr(stderr: &str) -> String {
    let trimmed = stderr.trim();
    let lower = trimmed.to_lowercase();

    if lower.contains("device offline") || lower.contains("no devices/emulators found") {
        "Device offline".to_string()
    } else if lower.contains("unauthorized") {
        "Device unauthorized — accept the USB debugging prompt on the device".to_string()
    } else if lower.contains("permission denied") {
        "Permission denied".to_string()
    } else if lower.contains("device not found") {
        "Device not found".to_string()
    } else if trimmed.is_empty() {
        "adb command failed".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_offline_device() {
        assert_eq!(
            classify_adb_stderr("error: device offline"),
            "Device offline"
        );
    }

    #[test]
    fn classify_unauthorized_device() {
        assert_eq!(
            classify_adb_stderr("device unauthorized.\nThis adb server's ..."),
            "Device unauthorized — accept the USB debugging prompt on the device"
        );
    }

    #[test]
    fn classify_permission_denied() {
        assert_eq!(
            classify_adb_stderr("logcat: Permission denied"),
            "Permission denied"
        );
    }
}
