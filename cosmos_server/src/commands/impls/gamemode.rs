use crate::commands::CommandSender;

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
        if ev.args.len() < 1 {
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

        return Ok(GamemodeCommand { receiver, gamemode });
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<GamemodeCommand, _>(
        ServerCommand::new("cosmos:gamemode", "[gamemode] (player)", "Sets the player to this gamemode."),
        app,
        |q_players: Query<(Entity, &Player)>, mut commands: Commands, mut evr_command: EventReader<CommandEvent<GamemodeCommand>>| {
            for ev in evr_command.read() {
                let Some(ent) = (match &ev.command.receiver {
                    Receiver::Name(name) => q_players.iter().find(|x| x.1.name() == name).map(|x| x.0),
                    Receiver::Entity(e) => Some(*e),
                }) else {
                    error!("Unable to find player {:?}", ev.command.receiver);
                    continue;
                };

                match ev.command.gamemode {
                    GameMode::Survival => {
                        commands.entity(ent).remove::<Creative>();
                        info!("Swapped {ent:?} to survival.");
                    }
                    GameMode::Creative => {
                        commands.entity(ent).insert(Creative);
                        info!("Swapped {ent:?} to creative.");
                    }
                }
            }
        },
    );
}
