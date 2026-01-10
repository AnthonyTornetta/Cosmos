use bevy::prelude::*;
use cosmos_core::{
    chat::ServerSendChatMessageMessage,
    entities::{
        health::{Dead, Health},
        player::Player,
    },
    netty::sync::events::server_event::NettyMessageWriter,
};

use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;

struct KillCommand {
    player: Option<String>,
}

impl CosmosCommandType for KillCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(Self {
            player: ev.args.first().cloned(),
        })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<KillCommand, _>(
        ServerCommand::new(
            "cosmos:kill",
            "(player)",
            "Kills the specified player or yourself if no player is specified",
        ),
        app,
        |mut evr_blueprint: MessageReader<CommandMessage<KillCommand>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         q_player: Query<(Entity, &Player)>,
         mut nevw_send_chat_msg: NettyMessageWriter<ServerSendChatMessageMessage>,
         mut commands: Commands| {
            for ev in evr_blueprint.read() {
                let player_name = &ev.command.player;

                let (ent, player) = if let Some(player_name) = player_name {
                    let Some((ent, player)) = q_player
                        .iter()
                        .find(|(_, player)| player.name().to_lowercase() == player_name.to_lowercase())
                    else {
                        ev.sender
                            .write(format!("Unable to find player {player_name}"), &mut evw_send_message);
                        continue;
                    };

                    (ent, player)
                } else if let Some(sender) = ev.sender.entity() {
                    if let Ok((ent, player)) = q_player.get(sender) {
                        (ent, player)
                    } else {
                        ev.sender.write("Invalid player!".to_string(), &mut evw_send_message);

                        continue;
                    }
                } else {
                    ev.sender
                        .write("You must specify the player to kill!".to_string(), &mut evw_send_message);

                    continue;
                };

                ev.sender.write(format!("Killing {}!", player.name()), &mut evw_send_message);

                nevw_send_chat_msg.broadcast(ServerSendChatMessageMessage {
                    sender: None,
                    message: format!("{} was killed!", player.name()),
                });

                commands.entity(ent).insert((Dead, Health::new(0)));
            }
        },
    );
}
