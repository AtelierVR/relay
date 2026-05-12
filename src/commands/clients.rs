use super::{Command, CommandContext};

pub struct ClientsCommand;

impl Command for ClientsCommand {
    fn name(&self) -> &str {
        "clients"
    }

    fn description(&self) -> &str {
        "List connected clients"
    }

    fn execute(&self, context: &CommandContext, _args: &[&str]) {
        let count = context.clients.count();
        if count == 0 {
            tracing::info!("No clients connected");
            return;
        }

        tracing::info!("Connected Clients ({}):", count);
        context.clients.for_each(|id, client| {
            let cli = client.read();
            let user_name = cli
                .user
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("Guest");
            tracing::info!(
                "  [{}] {} - {} ({})",
                id,
                user_name,
                cli.platform,
                cli.engine
            );
        });
    }
}
