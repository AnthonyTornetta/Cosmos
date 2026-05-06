use bevy::prelude::*;
use cosmos_core::physics::location::{Location, Sector, SectorUnit};
use walkdir::WalkDir;

use crate::{
    commands::SendCommandMessageMessage,
    persistence::{
        loading::{NeedsBlueprintLoaded, load_blueprint},
        saving::NeedsBlueprinted,
    },
};

use super::super::prelude::*;

struct ResaveAllBpsCommand {
    spawn_at: Location,
    root: String,
}

impl CosmosCommandType for ResaveAllBpsCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() < 4 {
            return Err(ArgumentError::TooFewArguments);
        }

        let root = ev.args[0].to_owned();

        let mut spawn_at = Location::default();

        if ev.args.len() >= 4 {
            let x = ev.args[1].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 1,
                type_name: "SectorUnit".into(),
            })?;
            let y = ev.args[2].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 2,
                type_name: "SectorUnit".into(),
            })?;
            let z = ev.args[3].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 3,
                type_name: "SectorUnit".into(),
            })?;

            spawn_at.sector = Sector::new(x, y, z);

            if ev.args.len() == 7 {
                let x = ev.args[4].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 4,
                    type_name: "SectorUnit".into(),
                })?;
                let y = ev.args[5].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 5,
                    type_name: "SectorUnit".into(),
                })?;
                let z = ev.args[6].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                    arg_index: 6,
                    type_name: "SectorUnit".into(),
                })?;
                spawn_at.local = Vec3::new(x, y, z);
            } else if ev.args.len() != 7 {
                return Err(ArgumentError::TooFewArguments);
            }
        } else if ev.args.len() != 4 {
            return Err(ArgumentError::TooFewArguments);
        }

        Ok(ResaveAllBpsCommand { spawn_at, root })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<ResaveAllBpsCommand, _>(
        ServerCommand::new(
            "cosmos:resave-all-blueprints",
            "root ([x], [y], [z]) ([x], [y], [z])",
            "Resaves every blueprint by loading them in and triggering a save. For updating the game only.",
        ),
        app,
        |mut evr_load: MessageReader<CommandMessage<ResaveAllBpsCommand>>,
         mut commands: Commands,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>| {
            for ev in evr_load.read() {
                for entry in WalkDir::new(&ev.command.root)
                    .into_iter()
                    .flatten()
                    .filter(|entry| entry.path().is_file())
                    .filter(|entry| entry.file_name().to_str().map(|x| x.ends_with(".bp")).unwrap_or(false))
                {
                    const SCATTER: f32 = 10_000.0;
                    let offset = Vec3::new(
                        rand::random::<f32>() * SCATTER - SCATTER / 2.0,
                        rand::random::<f32>() * SCATTER - SCATTER / 2.0,
                        rand::random::<f32>() * SCATTER - SCATTER / 2.0,
                    );

                    let mut end_name = entry.file_name().to_str().unwrap();
                    end_name = &end_name[..end_name.len() - ".bp".len()];

                    let Ok(bp) = load_blueprint(entry.path().to_str().unwrap()) else {
                        error!("Failed to read bp {:?}", entry.path());
                        continue;
                    };

                    commands.spawn((
                        ev.command.spawn_at + offset,
                        NeedsBlueprintLoaded {
                            spawn_at: ev.command.spawn_at + offset,
                            rotation: Quat::IDENTITY,
                            path: entry.path().to_str().unwrap().to_owned(),
                        },
                        NeedsBlueprinted {
                            name: bp.name().to_owned(),
                            blueprint_name: end_name.to_owned(),
                            blueprint_type: None,
                            override_path: Some(entry.path().to_str().unwrap().to_owned()),
                        },
                    ));
                }

                ev.sender
                    .write(format!("Resaving all bps @ {}", ev.command.root), &mut evw_send_message);
            }
        },
    );
}
