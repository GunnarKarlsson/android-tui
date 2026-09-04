mod donut;
mod insight;
mod logcat;
mod network;
mod ram;
mod storage;
mod traffic;

pub use insight::insight_panel;
pub use logcat::{logcat_all_panel, logcat_errors_panel};
pub use network::network_panel;
pub use ram::ram_gauge_panel;
pub use storage::{storage_gauge_panel, storage_usage_panel};
pub use traffic::protocols_panel;
