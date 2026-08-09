use super::{Command, CommandContext};
use tracing::{info, warn};

pub struct UseCommand;

impl Command for UseCommand {
    fn name(&self) -> &str {
        "use"
    }

    fn description(&self) -> &str {
        "Select an instance by internal ID for subsequent tps/threshold commands"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        if args.is_empty() {
            warn!("Usage: use <internal_id>");
            return;
        }

        let iid: u8 = match args[0].parse() {
            Ok(v) => v,
            Err(_) => {
                warn!("Invalid internal_id: '{}' (must be 0–254)", args[0]);
                return;
            }
        };

        let exists = context.instances.get(iid).is_some();
        if !exists {
            warn!("No instance with internal_id {iid}");
            return;
        }

        *context.selected_instance.lock() = Some(iid);
        info!("Selected instance #{iid}");
    }
}
