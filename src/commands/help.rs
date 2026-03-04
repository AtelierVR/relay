use super::{Command, CommandContext};

pub struct HelpCommand;

impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    
    fn description(&self) -> &str {
        "Show this help message"
    }
    
    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        tracing::info!("Available Commands:");
        let maxlen = context.commands_info.iter()
            .map(|cmd| cmd.name.len())
            .max()
            .unwrap_or(0);
        
        for cmd_info in &context.commands_info {
            tracing::info!("  {:<width$} - {}", cmd_info.name, cmd_info.description, width = maxlen);
        }
    }
}
