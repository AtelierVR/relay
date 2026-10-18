use super::{Command, CommandContext};
use tracing::warn;

pub struct StopCommand;

impl Command for StopCommand {
    fn name(&self) -> &str {
        "stop"
    }

    fn description(&self) -> &str {
        "Gracefully stop the relay server (alias: exit)"
    }

    fn aliases(&self) -> &[&str] {
        &["exit"]
    }

    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        warn!("[Command] Stop command received - initiating graceful shutdown");
        warn!("[Command] Relay shutdown initiated. Disconnecting all clients and stopping instances...");
        context.state.shutdown.notify_one();
    }
}
