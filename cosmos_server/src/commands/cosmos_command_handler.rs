//! Handles all the server console commands

use std::{
    fs::{self},
    path::Path,
};

use bevy::{
    app::Update,
    ecs::schedule::IntoSystemConfigs,
    log::warn,
    prelude::{App, Commands, Entity, EventReader, Name, Quat, Query, Res, ResMut, Startup, Vec3, With},
};
use cosmos_core::{
    chat::ServerSendChatMessageEvent,
    ecs::NeedsDespawned,
    netty::sync::events::server_event::NettyEventWriter,
    persistence::Blueprintable,
    physics::location::{Location, Sector, SectorUnit},
};
use thiserror::Error;

use crate::persistence::{
    loading::{LoadingSystemSet, NeedsBlueprintLoaded},
    saving::NeedsBlueprinted,
};

use super::{CosmosCommandInfo, CosmosCommandSent, CosmosCommands};

fn register_commands(mut commands: ResMut<CosmosCommands>) {
    commands.add_command_info(CosmosCommandInfo {
        name: "help".into(),
        usage: "help [command?]".into(),
        description: "Gets information about every command.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "ping".into(),
        usage: "ping".into(),
        description: "Says 'Pong'.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "blueprint".into(),
        usage: "blueprint [entity_id] [file_name]".into(),
        description: "blueprints the given structure to that file. Do not specify the file extension.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "blueprints".into(),
        usage: "blueprints {blueprint_type}".into(),
        description: "Lists all the blueprints available. The type is optional, and if provided will only list blueprints for that type."
            .into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "load".into(),
        usage: "load [blueprint_type] [blueprint_name] ([x], [y], [z]) ([x], [y], [z])".into(),
        description: "Loads the given structure from the file for that name. You can specify sector coords and the local coords to specify the coordinates to spawn it."
            .into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "list".into(),
        usage: "list".into(),
        description: "Lists all the savable entity ids".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "despawn".into(),
        usage: "despawn [entity_id]".into(),
        description: "Despawns the given entity.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "say".into(),
        usage: "say [...message]".into(),
        description: "Sends the given text to all connected players".into(),
    });
}

fn display_help(command_name: Option<&str>, commands: &CosmosCommands) {
    if let Some(command_name) = command_name {
        if let Some(info) = commands.command_info(command_name) {
            println!("=== {} ===", info.name);
            println!("\t{}\n\t{}", info.usage, info.description);

            return;
        }
    }

    println!("=== All Commands ===");
    for (_, info) in commands.commands() {
        println!("{}\n\t{}\n\t{}", info.name, info.usage, info.description);
    }
}

#[derive(Debug, Error)]
enum ArgumentError {
    #[error("Too few arguments: {0}")]
    TooFewArguments(String),
    // #[error("Too many arguments: {0}")]
    // TooManyArguments(String),
}

fn cosmos_command_listener(
    mut commands: Commands,
    mut command_events: EventReader<CosmosCommandSent>,
    cosmos_commands: Res<CosmosCommands>,
    mut nevw_send_chat_msg: NettyEventWriter<ServerSendChatMessageEvent>,
    all_blueprintable_entities: Query<(Entity, &Name, &Location), With<Blueprintable>>,
) {
    for ev in command_events.read() {
        match ev.name.as_str() {
            "help" => {
                if ev.args.len() != 1 {
                    display_help(None, &cosmos_commands);
                } else {
                    display_help(Some(&ev.args[0]), &cosmos_commands);
                }
            }
            "say" => {
                let message = ev.args.join(" ");

                nevw_send_chat_msg.broadcast(ServerSendChatMessageEvent { sender: None, message });
            }
            "ping" => {
                println!("Pong");
            }
            "list" => {
                println!("All blueprintable entities: ");
                println!("Name\tSector\t\tId");
                for (entity, name, location) in all_blueprintable_entities.iter() {
                    println!("{name}\t{}\t{} ", location.sector(), entity.to_bits());
                }
                println!("======================================")
            }
            "despawn" => {
                if ev.args.len() != 1 {
                    display_help(Some("despawn"), &cosmos_commands);
                } else if let Ok(index) = ev.args[0].parse::<u64>() {
                    if let Ok(entity) = Entity::try_from_bits(index) {
                        if let Some(mut entity_commands) = commands.get_entity(entity) {
                            entity_commands.insert(NeedsDespawned);
                            println!("Despawned entity {index}");
                        } else {
                            println!("Entity not found");
                        }
                    } else {
                        println!("Invalid entity id - {index}.");
                    }
                } else {
                    println!("This must be the entity's ID (positive whole number)");
                }
            }
            "load" => {
                if ev.args.len() < 2 || ev.args.len() > 8 {
                    display_help(Some("load"), &cosmos_commands);
                } else {
                    let path = format!("blueprints/{}/{}.bp", ev.args[0], ev.args[1]);

                    fn parse_args(ev: &CosmosCommandSent) -> anyhow::Result<Location> {
                        let mut spawn_at = Location::default();

                        if ev.args.len() >= 5 {
                            let x = ev.args[2].parse::<SectorUnit>()?;
                            let y = ev.args[3].parse::<SectorUnit>()?;

                            let z = ev.args[4].parse::<SectorUnit>()?;

                            spawn_at.sector = Sector::new(x, y, z);

                            if ev.args.len() == 8 {
                                let x = ev.args[5].parse::<f32>()?;
                                let y = ev.args[6].parse::<f32>()?;
                                let z = ev.args[7].parse::<f32>()?;
                                spawn_at.local = Vec3::new(x, y, z);
                            } else if ev.args.len() != 5 {
                                return Err(ArgumentError::TooFewArguments("Missing some local coordinate arguments".into()).into());
                            }
                        } else if ev.args.len() != 2 {
                            return Err(ArgumentError::TooFewArguments("Missing some sector coordinate arguments".into()).into());
                        }

                        Ok(spawn_at)
                    }

                    let Ok(spawn_at) = parse_args(ev).map_err(|e| warn!("{e}")) else {
                        continue;
                    };

                    commands.spawn((
                        spawn_at,
                        NeedsBlueprintLoaded {
                            spawn_at,
                            rotation: Quat::IDENTITY,
                            path,
                        },
                    ));
                }
            }
            "blueprint" => {
                if ev.args.len() != 2 {
                    display_help(Some("blueprint"), &cosmos_commands);
                    continue;
                }
                let Ok(index) = ev.args[0].parse::<u64>() else {
                    println!("The first argument must be the entity's index (positive number)");
                    continue;
                };

                let Ok(entity) = Entity::try_from_bits(index) else {
                    println!("Invalid entity index {index}");
                    continue;
                };

                if !all_blueprintable_entities.contains(entity) {
                    println!("This entity is not blueprintable!");
                    continue;
                };

                println!("Blueprinting entity!");

                commands.entity(entity).insert(NeedsBlueprinted {
                    blueprint_name: ev.args[1].to_owned(),
                    ..Default::default()
                });
            }
            "blueprints" => {
                let check_for = if ev.args.len() == 1 {
                    Some(ev.args[0].as_str())
                } else if ev.args.is_empty() {
                    None
                } else {
                    display_help(Some("blueprints"), &cosmos_commands);
                    continue;
                };

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

                    if check_for.map(|x| x == blueprint_type).unwrap_or(true) {
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
            _ => {
                display_help(Some(&ev.text), &cosmos_commands);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, register_commands)
        .add_systems(Update, cosmos_command_listener.before(LoadingSystemSet::BeginLoading));
}
