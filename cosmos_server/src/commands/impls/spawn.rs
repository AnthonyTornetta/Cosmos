use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    persistence::LoadingDistance,
    physics::location::{Location, systems::Anchor},
};

use crate::{
    commands::{
        CosmosCommandSent, SendCommandMessageMessage, ServerCommand,
        parser::location_parser::{CommandLocation, parse_location},
        prelude::{ArgumentError, CommandMessage, CosmosCommandType, create_cosmos_command},
    },
    persistence::saving::NeverSave,
};

struct SpawnCommand {
    spawn_type: String,
    location: CommandLocation,
}

impl CosmosCommandType for SpawnCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        let args = ev.args.as_slice();

        let spawn_type = args[0].clone();

        let loc = if args.is_empty() {
            Ok((CommandLocation::default(), Default::default()))
        } else {
            parse_location(&ev.args[1..])
        };

        let loc = match loc {
            Ok((loc, n)) => {
                if n != args.len() - 1 {
                    return Err(ArgumentError::TooManyArguments);
                }
                loc
            }
            Err(e) => return Err(e),
        };

        Ok(Self { spawn_type, location: loc })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SpawnCommand, _>(
        ServerCommand::new(
            "cosmos:spawn",
            "[entity_type] (entity_location)",
            "Spawns the given entity type at this location.",
        ),
        app,
        |mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut commands: Commands,
         q_loc: Query<&Location>,
         mut evr_command: MessageReader<CommandMessage<SpawnCommand>>| {
            for ev in evr_command.read() {
                let Some(loc) = ev.command.location.to_location(ev.sender.entity().and_then(|e| q_loc.get(e).ok())) else {
                    ev.sender
                        .write("Cannot use relative location on non-player!", &mut evw_send_message);
                    continue;
                };

                let mut spawn_type = ev.command.spawn_type.clone();
                if !spawn_type.contains(":") {
                    spawn_type = format!("cosmos:{spawn_type}");
                }

                match spawn_type.as_str() {
                    "cosmos:fake_anchor" => {
                        ev.sender
                            .write(format!("Spawning cosmos:fake_anchor @ {loc}!"), &mut evw_send_message);
                        commands.spawn((
                            Anchor,
                            Velocity::default(),
                            NeverSave,
                            LoadingDistance::new(6, 7),
                            Name::new("Fake Anchor"),
                            loc,
                        ));
                    }
                    unknown => {
                        ev.sender.write(format!("Unknown entity type `{unknown}`!"), &mut evw_send_message);
                        continue;
                    }
                }
            }
        },
    );
}
