use std::{sync::OnceLock, time::Instant};

#[cfg(target_os = "linux")]
use std::fs;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sysinfo::Disks;

#[cfg(not(target_os = "linux"))]
use sysinfo::System;

/// System resource snapshot sent to the MasterServer.
/// Field names match the TypeScript `RelaySpecs` interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecsData {
    /// CPU usage percentage of the relay process (0–100).
    pub c: f64,
    /// Memory usage: [used, total] in bytes.
    pub m: [u64; 2],
    /// Upload bandwidth: [bytes_per_sec, max_bytes_per_sec].
    pub u: [u64; 2],
    /// Download bandwidth: [bytes_per_sec, max_bytes_per_sec].
    pub d: [u64; 2],
    /// Storage usage: [used, total] in bytes.
    pub s: [u64; 2],
}

// ── Process CPU tracking ──────────────────────────────────────────────────────

#[allow(dead_code)]
struct ProcessCpuStats {
    utime: u64,
    stime: u64,
    total_time: u64,
}

struct ProcessCpuState {
    last_stats: Option<ProcessCpuStats>,
    last_clock_ticks: u64,
    cpu_percent: f64,
}

fn process_cpu_state() -> &'static Mutex<ProcessCpuState> {
    static CELL: OnceLock<Mutex<ProcessCpuState>> = OnceLock::new();
    CELL.get_or_init(|| {
        Mutex::new(ProcessCpuState {
            last_stats: None,
            last_clock_ticks: 0,
            cpu_percent: 0.0,
        })
    })
}

// ── Network rate tracking ─────────────────────────────────────────────────────

struct NetState {
    last_tx: u64,
    last_rx: u64,
    last_time: Option<Instant>,
    rate_tx: u64,
    rate_rx: u64,
}

fn net_state() -> &'static Mutex<NetState> {
    static CELL: OnceLock<Mutex<NetState>> = OnceLock::new();
    CELL.get_or_init(|| {
        Mutex::new(NetState {
            last_tx: 0,
            last_rx: 0,
            last_time: None,
            rate_tx: 0,
            rate_rx: 0,
        })
    })
}

// ── CPU ───────────────────────────────────────────────────────────────────────

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Read total system clock ticks from /proc/stat.
#[cfg(target_os = "linux")]
fn read_system_clock_ticks() -> Option<u64> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        if parts[0] == "cpu" {
            // Sum all time values: user, nice, system, idle, iowait, irq, softirq, steal
            let total: u64 = parts
                .iter()
                .skip(1)
                .filter_map(|s| s.parse::<u64>().ok())
                .sum();
            return Some(total);
        }
    }
    None
}

/// Read process CPU usage percentage.
/// On Linux: uses /proc/self/stat differential measurement.
/// Returns total CPU usage of the relay process across all cores (0–100).
fn read_process_cpu() -> f64 {
    #[cfg(target_os = "linux")]
    {
        // Read /proc/self/stat for process CPU times
        if let Ok(content) = fs::read_to_string("/proc/self/stat") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() > 14 {
                // Field 14 = utime (CPU time in user mode)
                // Field 15 = stime (CPU time in kernel mode)
                let utime = parts[13].parse::<u64>().unwrap_or(0);
                let stime = parts[14].parse::<u64>().unwrap_or(0);
                let process_time = utime + stime;

                // Get system clock ticks
                if let Some(system_ticks) = read_system_clock_ticks() {
                    let mut state = process_cpu_state().lock();

                    if let Some(ref last) = state.last_stats {
                        let process_delta = process_time.saturating_sub(last.total_time);
                        let system_delta = system_ticks.saturating_sub(state.last_clock_ticks);

                        if system_delta > 0 {
                            // CPU % = (process_delta / system_delta) * num_cpus * 100
                            let num_cores = num_cpus() as f64;
                            let cpu_usage =
                                (process_delta as f64 / system_delta as f64) * num_cores * 100.0;
                            state.cpu_percent = (cpu_usage * 100.0).round() / 100.0;
                            // Round to 2 decimals
                        }
                    }

                    state.last_stats = Some(ProcessCpuStats {
                        utime,
                        stime,
                        total_time: process_time,
                    });
                    state.last_clock_ticks = system_ticks;

                    return state.cpu_percent;
                }
            }
        }
    }

    0.0
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn read_memory() -> [u64; 2] {
    // Match C# behaviour: report the relay process's own working set, not system-wide usage.
    // Uses /proc/self/status VmRSS on Linux (equivalent to process.WorkingSet64 in C#).
    #[cfg(target_os = "linux")]
    {
        let total = read_total_memory_linux();

        if let Ok(content) = fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    let kb: u64 = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    return [kb * 1024, total];
                }
            }
        }
        [0, total]
    }
    // Non-Linux fallback.
    #[cfg(not(target_os = "linux"))]
    {
        let mut sys = System::new_all();
        sys.refresh_memory();
        [sys.used_memory(), sys.total_memory()]
    }
}

#[cfg(target_os = "linux")]
fn read_total_memory_linux() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                return kb * 1024;
            }
        }
    }
    8 * 1024 * 1024 * 1024 // fallback: 8 GB
}

// ── Network ───────────────────────────────────────────────────────────────────

fn read_proc_net_dev() -> Option<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        let content = fs::read_to_string("/proc/net/dev").ok()?;
        let mut tx_total = 0u64;
        let mut rx_total = 0u64;
        for line in content.lines().skip(2) {
            let line = line.trim();
            if line.starts_with("lo:") {
                continue;
            }
            let colon = line.find(':')?;
            let data = &line[colon + 1..];
            let parts: Vec<&str> = data.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }
            rx_total += parts[0].parse::<u64>().unwrap_or(0);
            tx_total += parts[8].parse::<u64>().unwrap_or(0);
        }
        return Some((tx_total, rx_total));
    }
    #[allow(unreachable_code)]
    None
}

fn get_max_network_bandwidth() -> u64 {
    #[cfg(target_os = "linux")]
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        let mut max_speed: u64 = 0;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let iface = name.to_string_lossy();
            if iface == "lo" {
                continue;
            }
            let speed_path = format!("/sys/class/net/{}/speed", iface);
            if let Ok(content) = fs::read_to_string(&speed_path) {
                if let Ok(mbps) = content.trim().parse::<i64>() {
                    if mbps > 0 {
                        max_speed = max_speed.max(mbps as u64 * 1_000_000 / 8);
                    }
                }
            }
        }
        if max_speed > 0 {
            return max_speed;
        }
    }
    125_000_000 // Fallback: 1 Gbps in bytes/s
}

/// Returns (tx_rate_bytes_per_sec, rx_rate_bytes_per_sec, max_bandwidth_bytes_per_sec).
fn read_network_rates() -> (u64, u64, u64) {
    let (total_tx, total_rx) = read_proc_net_dev().unwrap_or((0, 0));
    let mut net = net_state().lock();
    let now = Instant::now();

    let (rate_tx, rate_rx) = if let Some(last_time) = net.last_time {
        let elapsed = now.duration_since(last_time).as_secs_f64();
        if elapsed >= 1.0 {
            let tx = ((total_tx.saturating_sub(net.last_tx)) as f64 / elapsed) as u64;
            let rx = ((total_rx.saturating_sub(net.last_rx)) as f64 / elapsed) as u64;
            net.rate_tx = tx;
            net.rate_rx = rx;
            net.last_tx = total_tx;
            net.last_rx = total_rx;
            net.last_time = Some(now);
            (tx, rx)
        } else {
            (net.rate_tx, net.rate_rx)
        }
    } else {
        net.last_tx = total_tx;
        net.last_rx = total_rx;
        net.last_time = Some(now);
        (0, 0)
    };

    (rate_tx, rate_rx, get_max_network_bandwidth())
}

// ── Storage ───────────────────────────────────────────────────────────────────

fn read_storage() -> [u64; 2] {
    let disks = Disks::new_with_refreshed_list();
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    // Prefer disk whose mount point is the longest prefix of cwd.
    let best = disks
        .iter()
        .filter_map(|d| {
            let mp = d.mount_point().to_string_lossy().into_owned();
            if cwd.starts_with(&mp) {
                Some((mp.len(), d))
            } else {
                None
            }
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, d)| d)
        .or_else(|| disks.iter().next());

    if let Some(d) = best {
        let total = d.total_space();
        return [total.saturating_sub(d.available_space()), total];
    }
    [0, 100 * 1024 * 1024 * 1024]
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Collect a full system snapshot matching the TypeScript `RelaySpecs` interface.
pub fn get_specs() -> SpecsData {
    let cpu = read_process_cpu();
    let mem = read_memory();
    let (rate_tx, rate_rx, max_bw) = read_network_rates();
    let storage = read_storage();

    SpecsData {
        c: cpu,
        m: mem,
        u: [rate_tx, max_bw],
        d: [rate_rx, max_bw],
        s: storage,
    }
}
