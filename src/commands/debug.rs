use super::{Command, CommandContext};
use crate::utils::logging;
use tracing::{info, warn};

// ── debug [true|false|toggle] ────────────────────────────────────────────────

pub struct DebugCommand;

impl Command for DebugCommand {
    fn name(&self) -> &str {
        "debug"
    }

    fn description(&self) -> &str {
        "Show or change debug logging: debug [true|false|toggle]"
    }

    fn execute(&self, _context: &CommandContext, args: &[&str]) {
        let Some(raw) = args.first() else {
            info!(
                "Debug logging is {}.",
                if logging::is_debug() {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            return;
        };

        match raw.to_ascii_lowercase().as_str() {
            "true" | "on" | "enable" | "enabled" | "1" => {
                logging::set_debug(true);
            }
            "false" | "off" | "disable" | "disabled" | "0" => {
                logging::set_debug(false);
            }
            "toggle" => {
                logging::toggle_debug();
            }
            _ => warn!("Invalid value '{raw}'. Usage: debug [true|false|toggle]"),
        }
    }
}
