use super::{Command, CommandContext};
use crate::constants::{ENGINE, PROTOCOL_VERSION, VERSION};

pub struct VersionCommand;

impl Command for VersionCommand {
    fn name(&self) -> &str {
        "version"
    }
    
    fn description(&self) -> &str {
        "Show relay version and engine info"
    }
    
    fn execute(&self, _context: &CommandContext, _args: &[&str]) {
        tracing::info!("Relay Version:");
        tracing::info!("  Engine: {} v{}", ENGINE, VERSION);
        tracing::info!("  Protocol: {}", PROTOCOL_VERSION);
    }
}
