use super::{Command, CommandContext};
use crate::instance::{InstanceFlags, TravelTarget};
use tracing::{info, warn};

/// `instances` — list every running instance, `instances <id>` — details of one.
pub struct InstancesCommand;

impl Command for InstancesCommand {
    fn name(&self) -> &str {
        "instances"
    }

    fn description(&self) -> &str {
        "List all running instances, or show the details of one (instances <id>)"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        if let Some(query) = args.first() {
            match query.parse::<u8>() {
                Ok(id) => show_instance(context, id),
                Err(_) => info!("Usage: instances [<id>]"),
            }
            return;
        }

        let count = context.instances.count();
        if count == 0 {
            info!("No instances running");
            return;
        }

        info!("Running Instances ({}):", count);
        context.instances.for_each(|id, instance| {
            let inst = instance.read();
            let player_count = inst.player_count();
            let world_id = inst.world.to_identifier();
            info!(
                "  [{}] {} - {}/{} players - {:?}",
                id, world_id, player_count, inst.capacity, inst.flags
            );
        });
    }
}

fn show_instance(context: &CommandContext, id: u8) {
    let Some(instance) = context.instances.get(id) else {
        warn!("No instance with id {}", id);
        return;
    };

    let inst = instance.read();
    info!("Instance {}:", id);
    info!("  Master ID:        {}", inst.master_id);
    info!("  Flags:            {:?}", inst.flags);
    info!(
        "  Players:          {}/{} (capacity 0 = unlimited)",
        inst.player_count(),
        inst.capacity
    );
    info!(
        "  Password:         {}",
        yes_no(inst.flags.contains(InstanceFlags::USE_PASSWORD))
    );
    info!(
        "  TPS:              {} (threshold {:.4}, render {:.1})",
        inst.tps, inst.threshold, inst.render_entity
    );
    info!("  Last tick:        {:.2} ms", inst.last_tick_duration);
    info!(
        "  Property resend:  {} s (0 = disabled)",
        inst.property_resend_interval
    );
    info!("  Entities:         {}", inst.entities.iter().count());

    match &inst.world {
        TravelTarget::Node(world) => {
            info!(
                "  World:            node {}?v={}@{}",
                world.master_id, world.version, world.address
            );
        }
        TravelTarget::Bundle(bundle) => {
            info!(
                "  World:            bundle {} ({} bytes, hash {})",
                bundle.url,
                bundle.size,
                hex::encode(&bundle.hash)
            );
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
