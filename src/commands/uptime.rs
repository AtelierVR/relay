use super::{Command, CommandContext};

pub struct UptimeCommand;

impl Command for UptimeCommand {
    fn name(&self) -> &str {
        "uptime"
    }
    
    fn description(&self) -> &str {
        "Show relay uptime"
    }
    
    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        
        let uptime_secs = (now_ms - context.start_time_ms) / 1000;
        let hours = uptime_secs / 3600;
        let minutes = (uptime_secs % 3600) / 60;
        let seconds = uptime_secs % 60;
        
        tracing::info!("Relay uptime: {}h {}m {}s", hours, minutes, seconds);
    }
}
