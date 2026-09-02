show we a whole implementation that fetches tcpdump via adb, then parses it and classifies it into HTTP, HTTPS, UDP and TCP (and other protocols)Here is a complete, production-ready Rust implementation.This implementation handles the biggest hurdle when capturing from Android (adb shell tcpdump -i any): it automatically detects whether tcpdump formats the stream using Standard Ethernet or Linux Cooked Capture (SLL) headers (which Android defaults to when monitoring all interfaces simultaneously).1. Project SetupAdd the required dependencies to your Cargo.toml. We will use pcap-parser to slice the incoming binary stream safely and etherparse for blazing-fast zero-copy packet header matching.toml[package]
name = "adb_analyzer"
version = "0.1.0"
edition = "2021"

[dependencies]
pcap-parser = "0.16"
etherparse = "0.16"
Use code with caution.2. The Complete Rust Source (src/main.rs)rustuse etherparse::SlicedPacket;
use pcap_parser::{PcapBlockOwned, PcapReader, PcapError, Linktype};
use std::io::ErrorKind;
use std::process::{Command, Stdio};

#[derive(Default, Debug)]
struct ByteCounters {
    total: u64,
    http: u64,
    https: u64,
    tcp_other: u64,
    udp: u64,
    other: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Launch tcpdump via ADB. We use "su -c" because raw captures require root on Android.
    // "-w -" streams raw pcap blocks straight into stdout.
    let mut child = Command::new("adb")
        .args(&["shell", "su", "-c", "tcpdump -i any -w -"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start ADB. Is it in your PATH and is your device connected?");

    let stdout = child.stdout.take().expect("Failed to bind to stdout");
    println!("🚀 Listening to Android traffic via ADB... Press Ctrl+C to terminate.\n");

    let mut counters = ByteCounters::default();
    
    // Track the link-layer protocol configuration from the PCAP file header.
    let mut detected_linktype = Linktype::ETHERNET; 

    // Create a buffered PcapReader to ingest chunks of bytes from stdout
    let mut pcap_reader = PcapReader::new(65536, stdout)?;

    loop {
        match pcap_reader.next() {
            Ok((offset, block)) => {
                match block {
                    PcapBlockOwned::LegacyHeader(header) => {
                        // Store link type (e.g. Ethernet vs Linux Cooked Capture) to choose the right parser downstream
                        detected_linktype = header.network;
                    }
                    PcapBlockOwned::Packet(packet) => {
                        let packet_len = packet.data.len() as u64;
                        counters.total += packet_len;

                        // 2. Separate processing based on LinkType (Android '-i any' usually results in SLL)
                        let parsed_packet = match detected_linktype {
                            Linktype::ETHERNET => SlicedPacket::from_ethernet(packet.data).ok(),
                            Linktype::LINUX_SLL => SlicedPacket::from_linux_sll(packet.data).ok(),
                            _ => None,
                        };

                        if let Some(sliced) = parsed_packet {
                            if let Some(transport) = sliced.transport {
                                match transport {
                                    etherparse::TransportSlice::Tcp(tcp) => {
                                        let sport = tcp.source_port();
                                        let dport = tcp.destination_port();

                                        if sport == 80 || dport == 80 {
                                            counters.http += packet_len;
                                        } else if sport == 443 || dport == 443 {
                                            // Optional: You could read sliced.payload here to verify 
                                            // the TLS handshake header signature [0x16, 0x03]
                                            counters.https += packet_len;
                                        } else {
                                            counters.tcp_other += packet_len;
                                        }
                                    }
                                    etherparse::TransportSlice::Udp(_) => {
                                        counters.udp += packet_len;
                                    }
                                    _ => {
                                        counters.other += packet_len;
                                    }
                                }
                            } else {
                                // Packet has no recognizable L4 Transport Layer (e.g. ICMP or raw IP)
                                counters.other += packet_len;
                            }
                        } else {
                            // Unparseable Link-layer frame
                            counters.other += packet_len;
                        }

                        // 3. Dynamic terminal logging
                        print!(
                            "\r[STATS] Total: {:10} B | HTTPS: {:10} B | HTTP: {:10} B | TCP (Other): {:10} B | UDP: {:10} B | Other: {:10} B",
                            counters.total, counters.https, counters.http, counters.tcp_other, counters.udp, counters.other
                        );
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                    _ => {}
                }
                // Free the parsed section from the internal buffer pipeline
                pcap_reader.consume(offset);
            }
            Err(PcapError::Incomplete) => {
                // The stream buffer is momentarily empty; block slightly and wait for more ADB stdout input
                match pcap_reader.refill() {
                    Ok(_) => continue,
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        println!("\n🛑 Stream ended. ADB connection closed.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("\n⚠️ Read buffer injection error: {:?}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("\n❌ Critical PCAP framing error: {:?}", e);
                break;
            }
        }
    }

    // Clean up child process context cleanly on drop/exit
    let _ = child.kill();
    Ok(())
}
Use code with caution.Running the ProjectPlug your Android device in via USB and ensure USB Debugging is toggled on.Run adb devices in your command line terminal to confirm your computer pairs with the target device.Build and launch your analyzer with optimizations enabled for better packet ingestion:bashcargo run --release
Use code with caution.If you plan to deploy or scale this tool, would you like to see how to format these metrics into a structured format like JSON output, or how to bundle it into a background thread so your main application UI can display it dynamically?