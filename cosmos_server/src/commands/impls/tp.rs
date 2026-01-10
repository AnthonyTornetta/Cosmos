use crate::commands::{
    CosmosCommandSent, SendCommandMessageMessage, ServerCommand,
    parser::location_parser::{CommandLocation, parse_location},
    prelude::{ArgumentError, CommandMessage, CosmosCommandType, create_cosmos_command},
};
use bevy::prelude::*;
use cosmos_core::{
    entities::player::{Player, teleport::TeleportMessage},
    netty::{netty_rigidbody::NettyRigidBodyLocation, sync::events::server_event::NettyMessageWriter},
    physics::location::Location,
};

enum TpLocation {
    Player(String),
    Position(CommandLocation),
}

struct TeleportCommand {
    loc: TpLocation,
    target: Option<String>,
}

impl CosmosCommandType for TeleportCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        let mut args = ev.args.as_slice();

        if args.len() == 1 {
            return Ok(TeleportCommand {
                loc: TpLocation::Player(args[0].to_owned()),
                target: None,
            });
        }

        if args.len() == 2 {
            return Ok(TeleportCommand {
                loc: TpLocation::Player(args[1].to_owned()),
                target: Some(args[0].to_owned()),
            });
        }

        let target = if args.len() == 4 || args.len() == 7 {
            let target = args[0].to_owned();
            args = &args[1..];
            Some(target)
        } else {
            None
        };

        let loc = match parse_location(args) {
            Ok((loc, n)) => {
                if n != args.len() {
                    return Err(ArgumentError::TooManyArguments);
                }
                loc
            }
            Err(e) => return Err(e),
        };

        Ok(TeleportCommand {
            loc: TpLocation::Position(loc),
            target,
        })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<TeleportCommand, _>(
        ServerCommand::new(
            "cosmos:tp",
            "(target) [player/location]",
            "Spawns the given entity type at this location.",
        ),
        app,
        |mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         q_loc: Query<&Location>,
         q_player: Query<(Entity, &Player)>,
         mut nmw_teleport: NettyMessageWriter<TeleportMessage>,
         mut evr_command: MessageReader<CommandMessage<TeleportCommand>>| {
            for ev in evr_command.read() {
                let (target_ent, target) = if let Some(target) = &ev.command.target {
                    let player = q_player.iter().find(|(_, p)| p.name().to_lowercase() == target.to_lowercase());
                    let Some((ent, player)) = player else {
                        ev.sender
                            .write(format!("Unable to find target `{}`", target), &mut evw_send_message);
                        continue;
                    };
                    (ent, player)
                } else {
                    let Some(ent) = ev.sender.entity() else {
                        ev.sender.write("Must specify a target", &mut evw_send_message);
                        continue;
                    };
                    let Ok(player) = q_player.get(ent).map(|x| x.1) else {
                        ev.sender.write("Invalid target entity", &mut evw_send_message);
                        continue;
                    };
                    (ent, player)
                };

                let loc = match &ev.command.loc {
                    TpLocation::Player(p_name) => {
                        let player = q_player.iter().find(|(_, p)| p.name().to_lowercase() == p_name.to_lowercase());
                        let Some((ent, _)) = player else {
                            ev.sender
                                .write(format!("Could not find player `{}`", p_name), &mut evw_send_message);
                            continue;
                        };
                        NettyRigidBodyLocation::Relative(Vec3::ZERO, ent)
                    }
                    TpLocation::Position(cmd_loc) => {
                        let Ok(loc) = q_loc.get(target_ent) else {
                            error!("Invalid location for tp target!");
                            ev.sender
                                .write("Something bad happened whiple executing this command", &mut evw_send_message);

                            continue;
                        };
                        NettyRigidBodyLocation::Absolute(cmd_loc.to_location(Some(loc)).expect("Impossible for this to fail"))
                    }
                };

                ev.sender.write(format!("Teleporting {}!", target.name()), &mut evw_send_message);

                nmw_teleport.write(TeleportMessage { to: loc }, target.client_id());
            }
        },
    );
}
