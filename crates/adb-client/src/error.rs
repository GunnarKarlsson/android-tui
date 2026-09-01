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
