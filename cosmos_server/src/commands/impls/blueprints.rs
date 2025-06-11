use std::{fs, path::Path};

use bevy::prelude::*;

use super::super::prelude::*;

struct BlueprintsCommand(Option<String>);

impl CosmosCommandType for BlueprintsCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(BlueprintsCommand(ev.args.first().cloned()))
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<BlueprintsCommand, _>(
        ServerCommand::new(
            "cosmos:blueprints",
            "{blueprint_type}",
            "Lists all the blueprints available. The type is optional, and if provided will only list blueprints for that type",
        ),
        app,
        |mut evr_blueprint: EventReader<CommandEvent<BlueprintsCommand>>| {
            for ev in evr_blueprint.read() {
                let Ok(files) = fs::read_dir("./blueprints") else {
                    println!("No blueprints yet!");
                    continue;
                };

                for blueprint_type in files {
                    let Ok(blueprint_type_dir) = blueprint_type else {
                        continue;
                    };

                    let file_name = blueprint_type_dir.file_name();
                    let blueprint_type = file_name.to_str().expect("Unable to read string");

                    let check_for = &ev.command.0;

                    if check_for.as_ref().map(|x| x == blueprint_type).unwrap_or(true) {
                        println!("{blueprint_type}:");
                        let Ok(blueprints) = fs::read_dir(format!("./blueprints/{blueprint_type}")) else {
                            println!("Unable to list blueprints in this directory!");
                            continue;
                        };

                        let mut printed = false;
                        for blueprint in blueprints {
                            let Ok(blueprint) = blueprint else {
                                continue;
                            };

                            printed = true;

                            let blueprint = blueprint.file_name();
                            let file_name = Path::new(&blueprint).file_stem().expect("Unable to get file stem");
                            let file_name = file_name.to_str().expect("Unable to read string");

                            println!("\t{file_name}");
                        }

                        if !printed {
                            println!("\tNo blueprints of this type");
                        }
                    }
                }
            }
        },
    );
}
