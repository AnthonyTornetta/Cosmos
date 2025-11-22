use bevy::prelude::*;
use cosmos_core::physics::location::{Location, Sector, SectorUnit};

use crate::persistence::loading::NeedsBlueprintLoaded;

use super::super::prelude::*;

struct LoadCommand {
    spawn_at: Location,
    path: String,
}

impl CosmosCommandType for LoadCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() < 2 {
            return Err(ArgumentError::TooFewArguments);
        } else if ev.args.len() > 8 {
            return Err(ArgumentError::TooManyArguments);
        }

        let path = format!("blueprints/{}/{}.bp", ev.args[0], ev.args[1]);

        let mut spawn_at = Location::default();

        if ev.args.len() >= 5 {
            let x = ev.args[2].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 2,
                type_name: "SectorUnit".into(),
            })?;
            let y = ev.args[3].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 3,
                type_name: "SectorUnit".into(),
            })?;
            let z = ev.args[4].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 4,
                type_name: "SectorUnit".into(),
            })?;

            spawn_at.sector = Sector::new(x, y, z);

            if ev.args.len() == 8 {
                let x = ev.args[5].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 5,
                    type_name: "SectorUnit".into(),
                })?;
                let y = ev.args[6].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 6,
                    type_name: "SectorUnit".into(),
                })?;
                let z = ev.args[7].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 7,
                    type_name: "SectorUnit".into(),
                })?;
                spawn_at.local = Vec3::new(x, y, z);
            } else if ev.args.len() != 5 {
                return Err(ArgumentError::TooFewArguments);
            }
        } else if ev.args.len() != 2 {
            return Err(ArgumentError::TooFewArguments);
        }

        Ok(LoadCommand { spawn_at, path })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<LoadCommand, _>(
        ServerCommand::new(
            "cosmos:load",
            "[blueprint_type] [blueprint_name] ([x], [y], [z]) ([x], [y], [z])",
            "Loads the given structure from the file for that name. You can specify sector coords and the local coords to specify the coordinates to spawn it",
        ),
        app,
        |mut evr_load: MessageReader<CommandMessage<LoadCommand>>, mut commands: Commands| {
            for ev in evr_load.read() {
                commands.spawn((
                    ev.command.spawn_at,
                    NeedsBlueprintLoaded {
                        spawn_at: ev.command.spawn_at,
                        rotation: Quat::IDENTITY,
                        path: ev.command.path.clone(),
                    },
                ));

                info!("Loading {} at {}!", ev.command.path, ev.command.spawn_at);
            }
        },
    );
}
