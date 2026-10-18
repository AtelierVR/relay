use super::{find_client, format_age_ms, now_ms, Command, CommandContext};
use tracing::{info, warn};

/// `clients` — list every connected client, `clients <id|ip:port>` — details of one.
pub struct ClientsCommand;

impl Command for ClientsCommand {
    fn name(&self) -> &str {
        "clients"
    }

    fn description(&self) -> &str {
        "List connected clients, or show the details of one (clients <id|ip:port>)"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        if let Some(query) = args.first() {
            show_client(context, query);
            return;
        }

        let count = context.clients.count();
        if count == 0 {
            info!("No clients connected");
            return;
        }

        info!("Connected Clients ({}):", count);
        context.clients.for_each(|id, client| {
            let cli = client.read();
            let user_name = cli
                .user
                .as_ref()
                .map(|u| u.display_name.as_str())
                .unwrap_or("Guest");
            info!(
                "  [{}] {} - {}/{} - {}",
                id, user_name, cli.platform, cli.engine, cli.address
            );
        });
    }
}

fn show_client(context: &CommandContext, query: &str) {
    let Some((id, client)) = find_client(context, query) else {
        warn!("No client matching '{}'", query);
        return;
    };

    let cli = client.read();
    info!("Client {}:", id);
    info!("  Address:         {}", cli.address);
    if !cli.from.is_empty() && cli.from != cli.address {
        info!("  Connects from:   {}", cli.from);
    }
    info!(
        "  Platform:        {}",
        if cli.platform.is_empty() {
            "-"
        } else {
            cli.platform.as_str()
        }
    );
    info!(
        "  Engine:          {}",
        if cli.engine.is_empty() {
            "-"
        } else {
            cli.engine.as_str()
        }
    );
    info!("  Auth state:      {:?}", cli.auth_state);
    match &cli.user {
        Some(user) => {
            info!(
                "  User:            {} (\"{}\")",
                user.to_identifier(),
                user.display_name
            );
        }
        None => info!("  User:            guest / unauthenticated"),
    }
    info!(
        "  Connected since: {} ago",
        format_age_ms(now_ms() - cli.connected_at)
    );
    info!(
        "  Last packet:     {} ms ago",
        cli.last_seen.elapsed().as_millis()
    );
    info!(
        "  QUIC:            {} (rtt {} ms)",
        cli.conn.remote_address(),
        cli.conn.rtt().as_millis()
    );
    info!(
        "  Room:            {}",
        match cli.room {
            Some(iid) => format!("{iid}"),
            None => String::from("none"),
        }
    );

    let dropped = cli.limits.dropped;
    info!("  Over budget:     {dropped} dropped packet(s)");
}
