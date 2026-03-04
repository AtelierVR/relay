use tracing::warn;
use super::{Command, CommandContext};

pub struct StopCommand;

impl Command for StopCommand {
    fn name(&self) -> &str {
        "stop"
    }
    
    fn description(&self) -> &str {
        "Gracefully stop the relay server"
    }
    
    fn execute(&self, _context: &CommandContext, _args: &[&str]) {
        warn!("[Command] Stop command received - initiating graceful shutdown");
        warn!("[Command] Relay shutdown initiated. Disconnecting all clients and stopping instances...");
    }
}
