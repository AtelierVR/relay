use super::{Command, CommandContext};

pub struct InstancesCommand;

impl Command for InstancesCommand {
    fn name(&self) -> &str {
        "instances"
    }

    fn description(&self) -> &str {
        "List all running instances"
    }

    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        let count = context.instances.count();
        if count == 0 {
            tracing::info!("No instances running");
            return;
        }

        tracing::info!("Running Instances ({}):", count);
        context.instances.for_each(|id, instance| {
            let inst = instance.read();
            let player_count = inst.player_count();
            let world_id = inst.world.to_identifier();
            tracing::info!("  [{}] {} - {} players", id, world_id, player_count);
        });
    }
}
