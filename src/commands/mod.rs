use crate::{
    client::ClientManager, config::Config, handlers::context::AppState, instance::InstanceManager,
};
use parking_lot::Mutex;
use std::sync::Arc;

mod clients;
mod help;
mod instances;
mod memory;
mod restart;
mod status;
mod stop;
mod tps;
mod uptime;
mod use_instance;
mod version;

pub use clients::ClientsCommand;
pub use help::HelpCommand;
pub use instances::InstancesCommand;
pub use memory::MemoryCommand;
pub use restart::RestartCommand;
pub use status::StatusCommand;
pub use stop::StopCommand;
pub use tps::{ThresholdCommand, TpsCommand};
pub use uptime::UptimeCommand;
pub use use_instance::UseCommand;
pub use version::VersionCommand;

/// Command interface - each command implements this trait
pub trait Command {
    /// Get the command name (e.g., "help", "status")
    fn name(&self) -> &str;

    /// Get the command description
    fn description(&self) -> &str;

    /// Alternative names for this command (e.g. ["th"] for "threshold").
    fn aliases(&self) -> &[&str] {
        &[]
    }

    /// Execute the command with context and optional arguments (output via logging)
    fn execute(&self, context: &CommandContext, args: &[&str]);
}

/// Command metadata for help display
#[derive(Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
}

/// Context shared across all commands
pub struct CommandContext {
    pub config: Arc<Config>,
    pub clients: Arc<ClientManager>,
    pub instances: Arc<InstanceManager>,
    pub start_time_ms: i64,
    pub commands_info: Vec<CommandInfo>,
    /// Currently selected instance internal ID (set via `use` command).
    pub selected_instance: Mutex<Option<u8>>,
    /// Full application state (for emitting events, etc.).
    pub state: Arc<AppState>,
}

impl CommandContext {
    pub fn new(
        config: Arc<Config>,
        clients: Arc<ClientManager>,
        instances: Arc<InstanceManager>,
        start_time_ms: i64,
        commands_info: Vec<CommandInfo>,
        state: Arc<AppState>,
    ) -> Self {
        Self {
            config,
            clients,
            instances,
            start_time_ms,
            commands_info,
            selected_instance: Mutex::new(None),
            state,
        }
    }
}

/// Command registry that manages all available commands
pub struct CommandRegistry {
    pub context: Arc<CommandContext>,
    commands: Vec<Box<dyn Command + Send + Sync>>,
}

impl CommandRegistry {
    pub fn new(mut context: CommandContext) -> Self {
        // Create commands without context
        let commands: Vec<Box<dyn Command + Send + Sync>> = vec![
            Box::new(HelpCommand),
            Box::new(StopCommand),
            Box::new(RestartCommand),
            Box::new(StatusCommand),
            Box::new(InstancesCommand),
            Box::new(ClientsCommand),
            Box::new(UptimeCommand),
            Box::new(VersionCommand),
            Box::new(MemoryCommand),
            Box::new(StopCommand),
            Box::new(RestartCommand),
            Box::new(UseCommand),
            Box::new(TpsCommand),
            Box::new(ThresholdCommand),
        ];

        // Extract command metadata
        let commands_info: Vec<CommandInfo> = commands
            .iter()
            .map(|cmd| CommandInfo {
                name: cmd.name().to_string(),
                description: cmd.description().to_string(),
            })
            .collect();

        // Update context with commands info
        context.commands_info = commands_info;
        let ctx = Arc::new(context);

        Self {
            context: ctx,
            commands,
        }
    }

    /// Execute a command by name
    pub fn execute(&self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            tracing::error!("Error: Empty command");
            return;
        }

        let cmd_name = parts[0].to_lowercase();
        let args = &parts[1..];

        // Find and execute the command (check name + aliases)
        for cmd in &self.commands {
            if cmd.name() == cmd_name {
                cmd.execute(&self.context, args);
                return;
            }
            if cmd.aliases().contains(&cmd_name.as_str()) {
                cmd.execute(&self.context, args);
                return;
            }
        }

        tracing::warn!(
            "Unknown command: '{}'. Type 'help' for available commands.",
            cmd_name
        );
    }
}
