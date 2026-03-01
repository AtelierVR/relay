use serde::{Deserialize, Serialize};
use sysinfo::System;

/// System resource snapshot sent to the MasterServer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecsData {
    /// Number of logical CPU cores.
    pub cpu_count: usize,
    /// CPU usage percentage (0–100), averaged across all cores.
    pub cpu_usage: f32,
    /// Total RAM in bytes.
    pub memory_total: u64,
    /// Used RAM in bytes.
    pub memory_used: u64,
}

/// Collect a current snapshot of CPU and memory utilisation.
pub fn get_specs() -> SpecsData {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_count = sys.cpus().len();
    let cpu_usage = if cpu_count == 0 {
        0.0
    } else {
        sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32
    };

    SpecsData {
        cpu_count,
        cpu_usage,
        memory_total: sys.total_memory(),
        memory_used: sys.used_memory(),
    }
}
