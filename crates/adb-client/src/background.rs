use std::thread::JoinHandle;

use crossbeam_channel::Sender;

/// Signal a background worker to stop without blocking on `join`.
///
/// Dropping the join handle detaches the thread so app shutdown is not stalled
/// by in-flight adb commands.
pub(crate) fn signal_stop_and_detach(
    stop_tx: &Sender<()>,
    join_handle: &mut Option<JoinHandle<()>>,
) {
    let _ = stop_tx.send(());
    let _ = join_handle.take();
}
