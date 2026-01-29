use crate::{
    commands::SendCommandMessageMessage,
    netty::player_filtering::{BlacklistedReason, PlayerBlacklist},
};

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::entities::player::Player;
use renet::RenetServer;
use steamworks::SteamId;

struct BanCommand {
    receiver: String,
    message: Option<String>,
}

impl CosmosCommandType for BanCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        let receiver = ev.args[0].clone();
        let message = if ev.args.len() > 1 {
            Some(ev.args.iter().skip(1).map(|x| x.as_str()).collect::<Vec<&str>>().join(" "))
        } else {
            None
        };

        Ok(BanCommand { receiver, message })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<BanCommand, _>(
        ServerCommand::new(
            "cosmos:ban",
            "[player]",
            "Disconnects this player from the server and prevents them from rejoining",
        ),
        app,
        |q_players: Query<&Player>,
         mut server: ResMut<RenetServer>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut blacklist: ResMut<PlayerBlacklist>,
         mut evr_command: MessageReader<CommandMessage<BanCommand>>| {
            for ev in evr_command.read() {
                let Some(player) = q_players.iter().find(|x| x.name() == ev.command.receiver) else {
                    ev.sender
                        .write(format!("Unable to find player `{}`", ev.command.receiver), &mut evw_send_message);
                    continue;
                };

                server.disconnect(player.client_id());
                blacklist.add_player(
                    SteamId::from_raw(player.client_id()),
                    player.name().to_owned(),
                    ev.command.message.clone().map(BlacklistedReason::new),
                );
                ev.sender.write(format!("Banned {}.", player.name()), &mut evw_send_message);
            }
        },
    );
}
