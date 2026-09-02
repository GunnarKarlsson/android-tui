use std::collections::HashMap;
use std::sync::LazyLock;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use crossbeam_channel::{Receiver, Sender};
use regex::Regex;

use crate::adb::run_adb_for_serial;
use crate::error::AdbError;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_BACKOFF_INTERVAL: Duration = Duration::from_secs(5);
const BACKOFF_STEP: Duration = Duration::from_secs(1);

/// Android framework tag for UDP traffic.
const TAG_UDP: u32 = 0xffff_fff1;
/// Android framework tag for TCP traffic.
const TAG_TCP: u32 = 0xffff_fff2;

static PACKAGE_UID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^package:(\S+)\s+uid:(\d+)\s*$").expect("valid package uid regex")
});

static IDENT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"uid=(-?\d+)\s+set=(\S+)\s+tag=(0x[0-9a-fA-F]+)").expect("valid ident regex")
});

static HISTORY_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\brb=(\d+)\b.*\btb=(\d+)\b").expect("valid history regex")
});

/// Snapshot of layer-4 protocol traffic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolStats {
    pub tcp_bytes: u64,
    pub udp_bytes: u64,
    pub packages: HashMap<u32, Vec<String>>,
    pub timestamp: SystemTime,
}

/// Update from the protocol poller — either a successful snapshot or a transient error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolUpdate {
    Stats(ProtocolStats),
    Error(String),
}

/// Background poller for TCP/UDP byte totals.
pub struct ProtocolPoller {
    stop_tx: Sender<()>,
    join_handle: Option<JoinHandle<()>>,
}

impl ProtocolPoller {
    /// Polls protocol stats every two seconds.
    pub fn spawn(serial: &str) -> Result<(Receiver<ProtocolUpdate>, Self), AdbError> {
        Self::spawn_with_interval(serial, DEFAULT_POLL_INTERVAL)
    }

    pub fn spawn_with_interval(
        serial: &str,
        interval: Duration,
    ) -> Result<(Receiver<ProtocolUpdate>, Self), AdbError> {
        let (update_tx, update_rx) = crossbeam_channel::unbounded();
        let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
        let serial = serial.to_string();

        let join_handle = thread::spawn(move || {
            let mut poll_interval = interval;
            while stop_rx.try_recv().is_err() {
                match fetch_protocol_stats(&serial) {
                    Ok(stats) => {
                        poll_interval = interval;
                        if update_tx.send(ProtocolUpdate::Stats(stats)).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        poll_interval = next_backoff_interval(poll_interval);
                        if update_tx
                            .send(ProtocolUpdate::Error(err.user_message()))
                            .is_err()
                        {
                            break;
                        }
                    }
                }

                sleep_until_stop(&stop_rx, poll_interval);
            }
        });

        Ok((
            update_rx,
            ProtocolPoller {
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

impl Drop for ProtocolPoller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn fetch_protocol_stats(serial: &str) -> Result<ProtocolStats, AdbError> {
    let packages_out = run_adb_for_serial(serial, &["shell", "pm", "list", "packages", "-U"])?;
    let packages = parse_package_uids(&String::from_utf8_lossy(&packages_out.stdout));

    let dump = run_adb_for_serial(serial, &["shell", "dumpsys", "netstats", "--uid", "--tag"])?;
    let (tcp_bytes, udp_bytes) = parse_protocol_bytes(&String::from_utf8_lossy(&dump.stdout));

    Ok(ProtocolStats {
        tcp_bytes,
        udp_bytes,
        packages,
        timestamp: SystemTime::now(),
    })
}

fn parse_package_uids(text: &str) -> HashMap<u32, Vec<String>> {
    let mut packages: HashMap<u32, Vec<String>> = HashMap::new();
    for line in text.lines() {
        let Some(caps) = PACKAGE_UID.captures(line.trim()) else {
            continue;
        };
        let name = caps[1].to_string();
        let Ok(uid) = caps[2].parse::<u32>() else {
            continue;
        };
        packages.entry(uid).or_default().push(name);
    }
    packages
}

/// Sums received + transmitted bytes for TCP (`tag=0xfffffff2`) and UDP (`tag=0xfffffff1`).
fn parse_protocol_bytes(text: &str) -> (u64, u64) {
    let mut tcp_bytes = 0u64;
    let mut udp_bytes = 0u64;
    let mut current_tag: Option<u32> = None;
    let mut counting = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("ident=") {
            if let Some(caps) = IDENT_LINE.captures(trimmed) {
                let uid: i32 = caps[1].parse().unwrap_or(0);
                let set = &caps[2];
                current_tag = parse_tag(&caps[3]);
                counting = should_count(uid, set, current_tag);
            } else {
                current_tag = None;
                counting = false;
            }
            continue;
        }

        if !counting {
            continue;
        }

        if let Some(caps) = HISTORY_LINE.captures(trimmed) {
            let rb: u64 = caps[1].parse().unwrap_or(0);
            let tb: u64 = caps[2].parse().unwrap_or(0);
            let total = rb.saturating_add(tb);
            match current_tag {
                Some(TAG_TCP) => tcp_bytes = tcp_bytes.saturating_add(total),
                Some(TAG_UDP) => udp_bytes = udp_bytes.saturating_add(total),
                _ => {}
            }
        }
    }

    (tcp_bytes, udp_bytes)
}

fn parse_tag(raw: &str) -> Option<u32> {
    let hex = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))?;
    u32::from_str_radix(hex, 16).ok()
}

fn should_count(uid: i32, set: &str, tag: Option<u32>) -> bool {
    let Some(tag) = tag else {
        return false;
    };
    if tag != TAG_TCP && tag != TAG_UDP {
        return false;
    }
    if set.eq_ignore_ascii_case("ALL") {
        uid == -1
    } else {
        true
    }
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

fn next_backoff_interval(current: Duration) -> Duration {
    (current + BACKOFF_STEP).min(MAX_BACKOFF_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGES_SAMPLE: &str = r#"package:com.google.android.youtube uid:10166
package:com.android.simappdialog.auto_generated_rro_product__ uid:10060
package:com.android.externalstorage uid:10098
package:com.android.server.telecom uid:1000
package:android uid:1000
"#;

    const SYS_DUMP_SAMPLE: &str = r#"Xt stats:
  Pending bytes: 0
  History since boot:
  ident=[{type=0, ratType=3, subscriberId=310260..., metered=true, defaultNetwork=false, oemManaged=OEM_NONE, subId=1}] uid=-1 set=ALL tag=0x0
    NetworkStatsHistory: bucketDuration=3600
      st=1787544000 rb=24028 rp=75 tb=25282 tp=86 op=0
      st=1787547600 rb=847 rp=9 tb=0 tp=0 op=0
UID stats:
  ident=[{type=1, ratType=COMBINED, wifiNetworkKey="AndroidWifi"open, metered=false, defaultNetwork=true, oemManaged=OEM_NONE, subId=-1}] uid=10144 set=FOREGROUND tag=0x0
    NetworkStatsHistory: bucketDuration=3600
      st=1787544000 rb=63623203 rp=51606 tb=1594753 tp=9557 op=0
  ident=[{type=1, ratType=COMBINED, wifiNetworkKey="AndroidWifi"open, metered=false, defaultNetwork=true, oemManaged=OEM_NONE, subId=-1}] uid=10144 set=FOREGROUND tag=0xfffffff2
    NetworkStatsHistory: bucketDuration=3600
      st=1787544000 rb=24028 rp=75 tb=25282 tp=86 op=0
      st=1787547600 rb=100 rp=1 tb=50 tp=1 op=0
  ident=[{type=1, ratType=COMBINED, wifiNetworkKey="AndroidWifi"open, metered=false, defaultNetwork=true, oemManaged=OEM_NONE, subId=-1}] uid=10166 set=DEFAULT tag=0xfffffff1
    NetworkStatsHistory: bucketDuration=3600
      st=1787544000 rb=500 rp=5 tb=200 tp=2 op=0
  ident=[{type=0, ratType=3, subscriberId=310260..., metered=true, defaultNetwork=true, oemManaged=OEM_NONE, subId=1}] uid=10166 set=DEFAULT tag=0xfffffff1
    NetworkStatsHistory: bucketDuration=3600
      st=1787544000 rb=80 rp=2 tb=20 tp=1 op=0
"#;

    #[test]
    fn parse_package_list() {
        let packages = parse_package_uids(PACKAGES_SAMPLE);
        assert_eq!(
            packages.get(&10166),
            Some(&vec!["com.google.android.youtube".to_string()])
        );
        assert_eq!(packages.get(&1000).map(|names| names.len()), Some(2));
        assert!(packages
            .get(&1000)
            .unwrap()
            .contains(&"com.android.server.telecom".to_string()));
    }

    #[test]
    fn parse_tcp_udp_tags_from_sysdump() {
        let (tcp, udp) = parse_protocol_bytes(SYS_DUMP_SAMPLE);
        assert_eq!(tcp, 24_028 + 25_282 + 100 + 50);
        assert_eq!(udp, 500 + 200 + 80 + 20);
    }

    #[test]
    fn ignore_combined_tag_zero() {
        let dump = r#"
  ident=[{type=1}] uid=10144 set=FOREGROUND tag=0x0
    NetworkStatsHistory: bucketDuration=3600
      st=1 rb=999999 rp=1 tb=999999 tp=1 op=0
"#;
        let (tcp, udp) = parse_protocol_bytes(dump);
        assert_eq!(tcp, 0);
        assert_eq!(udp, 0);
    }

    #[test]
    fn skip_set_all_for_app_uids() {
        let dump = r#"
  ident=[{type=1}] uid=10144 set=ALL tag=0xfffffff2
    NetworkStatsHistory: bucketDuration=3600
      st=1 rb=1000 rp=1 tb=1000 tp=1 op=0
  ident=[{type=1}] uid=10144 set=FOREGROUND tag=0xfffffff2
    NetworkStatsHistory: bucketDuration=3600
      st=1 rb=40 rp=1 tb=10 tp=1 op=0
"#;
        let (tcp, udp) = parse_protocol_bytes(dump);
        assert_eq!(tcp, 50);
        assert_eq!(udp, 0);
    }
}
