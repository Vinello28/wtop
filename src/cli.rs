use crate::collectors::SystemCollector;
use crate::config::AppConfig;
use crate::model::{
    BatteryData, CpuData, DiskIoData, GpuData, MemoryData, NetworkData, ProcessItem, SystemSnapshot,
};
use crate::ui::gauge::{format_bytes, format_duration, format_speed};
use serde::Serialize;
use std::fmt::Write as _;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A one-shot sample is taken 500ms after a throwaway warm-up sample (one of
/// the app's own valid sampling-rate presets, see `AppConfig::cycle_sampling_rate`).
/// Every rate-based collector (CPU%, disk/network throughput, per-process
/// CPU%) computes its value as a delta against a previous reading, so a
/// single `collect()` right after `SystemCollector::new()` would report all
/// zeros -- most obviously for network, whose `prev_time` starts as `None`.
const WARMUP_INTERVAL: Duration = Duration::from_millis(500);

/// Processes shown in the plain-text snapshot; JSON always includes the
/// full list since a script can filter it itself.
const TEXT_PROCESS_LIMIT: usize = 15;

/// What `main` should do after argv has been parsed.
pub enum Action {
    /// No CLI flag was given: fall through to the normal interactive TUI.
    RunTui,
    /// A CLI action already ran to completion (or failed); the process
    /// should exit with this code without ever touching the terminal.
    Exit(i32),
}

/// Parses argv (already stripped of argv[0]) and, when it names a CLI action
/// instead of "launch the TUI", runs it to completion. Errors are printed
/// here as a plain one-line message rather than bubbling up as a `Result` --
/// `main`'s default `Result` error printing uses `{:?}`, which would dump
/// `io::Error`'s internal `Debug` representation instead of something a
/// script's stderr should show a person.
pub fn handle_args(args: &[String]) -> Action {
    let mut once = false;
    let mut json = false;

    for arg in args {
        match arg.as_str() {
            "--once" => once = true,
            "--json" => json = true,
            "-h" | "--help" => {
                print_usage();
                return Action::Exit(0);
            }
            other => {
                eprintln!("wtop: unrecognized argument '{other}' (try --help)");
                return Action::Exit(1);
            }
        }
    }

    if once || json {
        run_snapshot(json)
    } else {
        Action::RunTui
    }
}

fn print_usage() {
    println!("wtop {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    wtop                Launch the interactive TUI");
    println!("    wtop --once         Print a single plain-text snapshot and exit");
    println!("    wtop --json         Print a single JSON snapshot and exit");
    println!("    wtop -h, --help     Show this help message");
}

fn run_snapshot(json: bool) -> Action {
    let mut collector = SystemCollector::new();
    let cfg = AppConfig::default();

    let _ = collector.collect(&cfg, "");
    thread::sleep(WARMUP_INTERVAL);
    let snapshot = collector.collect(&cfg, "");

    if json {
        let output = SnapshotOutput::from(&snapshot);
        match serde_json::to_string_pretty(&output) {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("wtop: failed to serialize snapshot: {e}");
                return Action::Exit(1);
            }
        }
    } else {
        print!("{}", format_text_snapshot(&snapshot));
    }
    Action::Exit(0)
}

/// Serializable projection of a `SystemSnapshot` for `--json`. Built
/// separately rather than deriving `Serialize` on `SystemSnapshot` itself,
/// since that struct's `timestamp` is a monotonic `Instant` (not meaningful
/// to a script) -- this borrows everything else and adds a wall-clock time
/// instead.
#[derive(Serialize)]
struct SnapshotOutput<'a> {
    captured_unix_secs: u64,
    uptime_secs: u64,
    cpu: &'a CpuData,
    memory: &'a MemoryData,
    io: &'a DiskIoData,
    gpu: &'a GpuData,
    battery: &'a BatteryData,
    net: &'a NetworkData,
    processes: &'a [ProcessItem],
}

impl<'a> From<&'a SystemSnapshot> for SnapshotOutput<'a> {
    fn from(snapshot: &'a SystemSnapshot) -> Self {
        let captured_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            captured_unix_secs,
            uptime_secs: snapshot.uptime_secs,
            cpu: &snapshot.cpu,
            memory: &snapshot.memory,
            io: &snapshot.io,
            gpu: &snapshot.gpu,
            battery: &snapshot.battery,
            net: &snapshot.net,
            processes: &snapshot.processes,
        }
    }
}

fn format_text_snapshot(snapshot: &SystemSnapshot) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "wtop snapshot - uptime {}",
        format_duration(snapshot.uptime_secs)
    );
    let _ = writeln!(out);

    let _ = writeln!(
        out,
        "CPU      : {} - {:.1}% ({} cores)",
        non_empty(&snapshot.cpu.model_name, "unknown"),
        snapshot.cpu.global_pct,
        snapshot.cpu.cores.len()
    );

    let _ = writeln!(
        out,
        "RAM      : {} / {} used ({:.1}%), commit {:.1}%",
        format_bytes(snapshot.memory.used_bytes),
        format_bytes(snapshot.memory.total_bytes),
        snapshot.memory.usage_pct,
        snapshot.memory.commit_pct
    );

    for part in &snapshot.io.partitions {
        let _ = writeln!(
            out,
            "DISK {:<3} : {:.1}% used ({} free / {})",
            part.mount.trim_end_matches(['\\', '/']),
            part.usage_pct,
            format_bytes(part.free_bytes),
            format_bytes(part.total_bytes)
        );
    }
    let _ = writeln!(
        out,
        "DISK I/O : read {}, write {}",
        format_speed(snapshot.io.read_bytes_sec),
        format_speed(snapshot.io.write_bytes_sec)
    );

    let _ = writeln!(
        out,
        "GPU      : {} - {:.1}%, VRAM {}",
        non_empty(&snapshot.gpu.name, "unknown"),
        snapshot.gpu.utilization_pct,
        format_bytes(snapshot.gpu.dedicated_vram_used)
    );

    let _ = writeln!(
        out,
        "NETWORK  : {} - down {}, up {}",
        non_empty(&snapshot.net.active_iface, "unknown"),
        format_speed(snapshot.net.rx_bytes_sec),
        format_speed(snapshot.net.tx_bytes_sec)
    );

    if snapshot.battery.is_present {
        let state = if snapshot.battery.charging {
            "charging"
        } else if snapshot.battery.ac_online {
            "plugged in"
        } else {
            "on battery"
        };
        let _ = writeln!(
            out,
            "BATTERY  : {:.0}% - {}",
            snapshot.battery.percent, state
        );
    } else {
        let _ = writeln!(out, "BATTERY  : no battery detected");
    }

    let _ = writeln!(out);
    let total = snapshot.processes.len();
    let shown = total.min(TEXT_PROCESS_LIMIT);
    if total > shown {
        let _ = writeln!(out, "PROCESSES (showing {shown} of {total}):");
    } else {
        let _ = writeln!(out, "PROCESSES ({total}):");
    }
    let _ = writeln!(out, "  {:<8} {:>6} {:>10}  NAME", "PID", "CPU%", "MEM");
    for proc in snapshot.processes.iter().take(TEXT_PROCESS_LIMIT) {
        let _ = writeln!(
            out,
            "  {:<8} {:>5.1}% {:>10}  {}",
            proc.pid,
            proc.cpu_pct,
            format_bytes(proc.mem_bytes),
            proc.name
        );
    }

    out
}

fn non_empty<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    if s.is_empty() { fallback } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_exits_successfully() {
        let result = handle_args(&["--help".to_string()]);
        assert!(matches!(result, Action::Exit(0)));
    }

    #[test]
    fn no_flags_falls_through_to_tui() {
        assert!(matches!(handle_args(&[]), Action::RunTui));
    }

    #[test]
    fn unrecognized_flag_exits_with_error_code() {
        let result = handle_args(&["--bogus".to_string()]);
        assert!(matches!(result, Action::Exit(1)));
    }

    #[test]
    fn text_snapshot_reports_no_battery_when_absent() {
        let snapshot = SystemSnapshot::default();
        let text = format_text_snapshot(&snapshot);
        assert!(text.contains("BATTERY  : no battery detected"));
    }

    #[test]
    fn text_snapshot_reports_battery_state_when_present() {
        let snapshot = SystemSnapshot {
            battery: BatteryData {
                is_present: true,
                percent: 77.0,
                ac_online: false,
                charging: false,
                seconds_remaining: Some(1200),
                history: vec![],
            },
            ..Default::default()
        };
        let text = format_text_snapshot(&snapshot);
        assert!(text.contains("BATTERY  : 77% - on battery"));
    }

    #[test]
    fn text_snapshot_caps_process_list() {
        let processes = (0..20)
            .map(|i| ProcessItem {
                pid: i,
                name: format!("proc{i}.exe"),
                cpu_pct: 0.0,
                mem_bytes: 0,
                threads: 1,
            })
            .collect();
        let snapshot = SystemSnapshot {
            processes,
            ..Default::default()
        };
        let text = format_text_snapshot(&snapshot);
        assert!(text.contains("showing 15 of 20"));
        assert!(text.contains("proc14.exe"));
        assert!(!text.contains("proc15.exe"));
    }

    #[test]
    fn snapshot_output_serializes_to_json() {
        let snapshot = SystemSnapshot::default();
        let output = SnapshotOutput::from(&snapshot);
        let json = serde_json::to_string(&output).expect("serialize snapshot");
        assert!(json.contains("\"captured_unix_secs\""));
        assert!(json.contains("\"battery\""));
        assert!(json.contains("\"processes\""));
    }
}
