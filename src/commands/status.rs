use super::{Command, CommandContext};
use crate::constants::{ENGINE, VERSION};

pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }
    
    fn description(&self) -> &str {
        "Show relay status and statistics"
    }
    
    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        let instance_count = context.instances.count();
        let client_count = context.clients.count();
        let max_instances = context.config.max_instances;
        
        tracing::info!("Relay Status:");
        tracing::info!("  Running: Yes");
        tracing::info!("  Instances: {}/{}", instance_count, max_instances);
        tracing::info!("  Clients: {}", client_count);
        tracing::info!("  Engine: {} v{}", ENGINE, VERSION);
    }
}
