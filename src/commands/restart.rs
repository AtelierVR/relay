use super::{Command, CommandContext};
use tracing::warn;

pub struct RestartCommand;

impl Command for RestartCommand {
    fn name(&self) -> &str {
        "restart"
    }

    fn description(&self) -> &str {
        "Restart the relay server"
    }

    fn execute(&self, _context: &CommandContext, _args: &[&str]) {
        warn!("[Command] Restart command received");
        warn!("[Command] Relay restart not implemented yet. Use stop and manually restart the process.");
    }
}
