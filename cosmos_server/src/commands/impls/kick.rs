use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::entities::player::Player;
use renet::RenetServer;

struct KickCommand {
    receiver: String,
}

impl CosmosCommandType for KickCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        if ev.args.len() != 1 {
            return Err(ArgumentError::TooFewArguments);
        }
        let receiver = ev.args[0].clone();

        Ok(KickCommand { receiver })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<KickCommand, _>(
        ServerCommand::new("cosmos:kick", "[player]", "Disconnects this player from the server"),
        app,
        |q_players: Query<&Player>,
         mut server: ResMut<RenetServer>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut evr_command: MessageReader<CommandMessage<KickCommand>>| {
            for ev in evr_command.read() {
                let Some(player) = q_players.iter().find(|x| x.name() == ev.command.receiver) else {
                    ev.sender
                        .write(format!("Unable to find player `{}`", ev.command.receiver), &mut evw_send_message);
                    continue;
                };

                server.disconnect(player.client_id());
                ev.sender.write(format!("Kicked {}.", player.name()), &mut evw_send_message);
            }
        },
    );
}

