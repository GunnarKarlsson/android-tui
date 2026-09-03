//! Cluster, redact, and (later) comment on Android HUD snapshots.

mod fingerprint;
mod reduce;

pub use fingerprint::{generate_device_label, generate_fingerprint, redact};
pub use reduce::{build_snapshot, log_snapshot, InsightLine, InsightSnapshot, LevelMask};
