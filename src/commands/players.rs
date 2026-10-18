use super::{format_age_ms, now_ms, Command, CommandContext};
use crate::player::{PlayerFlags, PlayerStatus};
use tracing::{info, warn};

/// `players` — list every player, `players <id>` — details of one.
pub struct PlayersCommand;

impl Command for PlayersCommand {
    fn name(&self) -> &str {
        "players"
    }

    fn description(&self) -> &str {
        "List players, or show the details of a player (players <id>)"
    }

    fn execute(&self, context: &CommandContext, args: &[&str]) {
        match args.first() {
            None => list_players(context),
            Some(query) => match query.parse::<u32>() {
                Ok(id) => show_player(context, id),
                Err(_) => info!("Usage: players [<id>]"),
            },
        }
    }
}

fn list_players(context: &CommandContext) {
    let mut rows: Vec<(u8, u32, u16, String, PlayerStatus, PlayerFlags)> = Vec::new();
    context.instances.for_each(|iid, instance| {
        let inst = instance.read();
        for player in inst.get_players() {
            rows.push((
                iid,
                player.id,
                player.client_id,
                player.display.clone().unwrap_or_else(|| "-".to_string()),
                player.status,
                player.flags,
            ));
        }
    });

    if rows.is_empty() {
        info!("No players online");
        return;
    }

    info!("Players ({}):", rows.len());
    for (iid, id, client_id, player_display, status, flags) in rows {
        info!(
            "  [instance {}] player {} (client {}) {} - {:?} - {:?}",
            iid, id, client_id, player_display, status, flags
        );
    }
}

fn show_player(context: &CommandContext, id: u32) {
    let mut found = false;

    context.instances.for_each(|iid, instance| {
        let inst = instance.read();
        let Some(player) = inst.get_player(id) else {
            return;
        };

        found = true;
        info!("Player {} (instance {}):", player.id, iid);
        info!("  Client:           {}", player.client_id);
        info!(
            "  Display:          {}",
            player.display.as_deref().unwrap_or("-")
        );
        info!("  Status:           {:?}", player.status);
        info!("  Flags:            {:?}", player.flags);
        info!("  Entity kind:      {}", player.kind);
        info!("  Owner id:         {}", player.owner_id);
        info!(
            "  Entered since:    {} ago",
            format_age_ms(now_ms() - player.created_at)
        );
        match &player.avatar {
            Some(avatar) => info!(
                "  Avatar:           {}@{} v{} ({} parameter(s))",
                avatar.id,
                avatar.server,
                avatar.version,
                avatar.parameters.len()
            ),
            None => info!("  Avatar:           none"),
        }
        info!(
            "  TPS override:     {} (0 = instance default)",
            player.custom_tps
        );
        info!(
            "  Threshold:        {:.4} (0 = instance default)",
            player.custom_threshold
        );
        info!("  Transform parts:  {}", player.transforms.iter().count());
        if let Some(tr) = player.transforms.get(0) {
            info!(
                "  Position:         {:.2}, {:.2}, {:.2}",
                tr.position.x, tr.position.y, tr.position.z
            );
            info!(
                "  Rotation:         {:.3}, {:.3}, {:.3}, {:.3}",
                tr.rotation.x, tr.rotation.y, tr.rotation.z, tr.rotation.w
            );
            info!(
                "  Scale:            {:.2}, {:.2}, {:.2}",
                tr.scale.x, tr.scale.y, tr.scale.z
            );
        }
    });

    if !found {
        warn!("No player with id {}", id);
    }
}
