use super::{Command, CommandContext};
use sysinfo::System;

pub struct MemoryCommand;

impl Command for MemoryCommand {
    fn name(&self) -> &str {
        "memory"
    }

    fn description(&self) -> &str {
        "Show memory usage statistics"
    }

    fn execute(&self, _context: &CommandContext, _args: &[&str]) {
        let mut sys = System::new_all();
        sys.refresh_memory();

        let total_mb = sys.total_memory() / 1024;
        let used_mb = sys.used_memory() / 1024;
        let available_mb = sys.available_memory() / 1024;
        let percent = if total_mb > 0 {
            ((used_mb as f64 / total_mb as f64) * 100.0) as u32
        } else {
            0
        };

        tracing::info!("Memory Usage:");
        tracing::info!("  Used: {} MB / {} MB ({}%)", used_mb, total_mb, percent);
        tracing::info!("  Available: {} MB", available_mb);
    }
}
