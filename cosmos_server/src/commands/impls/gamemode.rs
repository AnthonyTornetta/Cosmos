use crate::commands::{CommandSender, SendCommandMessageMessage};

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::entities::player::{Player, creative::Creative};

enum GameMode {
    Survival,
    Creative,
}

#[derive(Debug)]
enum Receiver {
    Entity(Entity),
    Name(String),
}

struct GamemodeCommand {
    receiver: Receiver,
    gamemode: GameMode,
}

impl CosmosCommandType for GamemodeCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }
        if ev.args.len() > 2 {
            return Err(ArgumentError::TooManyArguments);
        }

        let gamemode = match ev.args[0].to_lowercase().as_str() {
            "s" | "survival" => GameMode::Survival,
            "c" | "creative" => GameMode::Creative,
            _ => {
                return Err(ArgumentError::InvalidType {
                    arg_index: 0,
                    type_name: "GameMode".into(),
                });
            }
        };

        let receiver = if ev.args.len() == 2 {
            Receiver::Name(ev.args[1].clone())
        } else {
            match ev.sender {
                CommandSender::Server => return Err(ArgumentError::TooFewArguments),
                CommandSender::Player(e) => Receiver::Entity(e),
            }
        };

        Ok(GamemodeCommand { receiver, gamemode })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<GamemodeCommand, _>(
        ServerCommand::new("cosmos:gamemode", "[gamemode] (player)", "Sets the player to this gamemode."),
        app,
        |q_players: Query<(Entity, &Player)>,
         mut commands: Commands,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut evr_command: MessageReader<CommandMessage<GamemodeCommand>>| {
            for ev in evr_command.read() {
                let Some((ent, player)) = (match &ev.command.receiver {
                    Receiver::Name(name) => q_players.iter().find(|x| x.1.name() == name),
                    Receiver::Entity(e) => q_players.get(*e).ok(),
                }) else {
                    ev.sender
                        .write(format!("Unable to find player {:?}", ev.command.receiver), &mut evw_send_message);
                    continue;
                };

                match ev.command.gamemode {
                    GameMode::Survival => {
                        commands.entity(ent).remove::<Creative>();
                        ev.sender
                            .write(format!("Swapped {} to survival.", player.name()), &mut evw_send_message);
                    }
                    GameMode::Creative => {
                        commands.entity(ent).insert(Creative);
                        ev.sender
                            .write(format!("Swapped {} to creative.", player.name()), &mut evw_send_message);
                    }
                }
            }
        },
    );
}
