use super::{Command, CommandContext};
use tracing::{info, warn};

// ── tps [value] ─────────────────────────────────────────────────────────────

pub struct TpsCommand;

impl Command for TpsCommand {
    fn name(&self) -> &str {
        "tps"
    }

    fn description(&self) -> &str {
        "Get or set the TPS of the selected instance"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        let sel = *context.selected_instance.lock();
        let Some(iid) = sel else {
            warn!("No instance selected. Use 'use <internal_id>' first.");
            return;
        };

        let Some(arc_inst) = context.instances.get(iid) else {
            warn!("Instance #{iid} no longer exists.");
            return;
        };

        let mut inst = arc_inst.write();

        if args.is_empty() {
            info!("Instance #{iid} TPS: {} (effective: {})",
                inst.tps,
                inst.get_effective_tps(&context.config.load_balancing));
        } else {
            let value: u8 = match args[0].parse() {
                Ok(v) if v > 0 => v,
                Ok(_) => {
                    warn!("TPS must be > 0, got '{}'", args[0]);
                    return;
                }
                Err(_) => {
                    warn!("Invalid TPS value: '{}'", args[0]);
                    return;
                }
            };
            inst.set_tps(value);
            info!("Instance #{iid} TPS set to {value}");
        }
    }
}

// ── threshold | th [value] ───────────────────────────────────────────────────

pub struct ThresholdCommand;

impl Command for ThresholdCommand {
    fn name(&self) -> &str {
        "threshold"
    }

    fn aliases(&self) -> &[&str] {
        &["th"]
    }

    fn description(&self) -> &str {
        "Get or set the transform threshold of the selected instance (alias: th)"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        let sel = *context.selected_instance.lock();
        let Some(iid) = sel else {
            warn!("No instance selected. Use 'use <internal_id>' first.");
            return;
        };

        let Some(arc_inst) = context.instances.get(iid) else {
            warn!("Instance #{iid} no longer exists.");
            return;
        };

        let mut inst = arc_inst.write();

        if args.is_empty() {
            info!("Instance #{iid} threshold: {} (effective: {})",
                inst.threshold,
                inst.get_effective_threshold(&context.config.load_balancing));
        } else {
            let value: f32 = match args[0].parse() {
                Ok(v) if v >= 0.0 => v,
                Ok(_) => {
                    warn!("Threshold must be >= 0, got '{}'", args[0]);
                    return;
                }
                Err(_) => {
                    warn!("Invalid threshold value: '{}'", args[0]);
                    return;
                }
            };
            inst.set_threshold(value);
            info!("Instance #{iid} threshold set to {value}");
        }
    }
}
