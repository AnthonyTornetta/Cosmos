use bevy::prelude::{
    App, Commands, DespawnRecursiveExt, Entity, EventReader, EventWriter, Query, Res, ResMut,
    Transform, With,
};
use cosmos_core::structure::{events::StructureCreated, planet::Planet, ship::Ship, Structure};

use crate::structure::saving::{
    load_structure, SaveStructure, SendDelayedStructureLoadEvent, StructureType,
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
        name: "save".into(),
        usage: "save [entity_id] [file_name]".into(),
        description: "Saves the given structure to that file. Do not specify the file extension."
            .into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "load".into(),
        usage: "load [structure_name] [planet_type] ([x], [y], [z])".into(),
        description: "Loads the given structure from the file for that name. The planet type should be either 'planet' or 'ship'. You can specify x/y/z to specify the coordinates to spawn it at."
            .into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "list".into(),
        usage: "list".into(),
        description: "Lists all entity bits with no parents (top-level)".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "despawn".into(),
        usage: "despawn [entity_id]".into(),
        description: "Despawns the given entity.".into(),
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

fn cosmos_command_listener(
    mut commands: Commands,
    mut command_events: EventReader<CosmosCommandSent>,
    cosmos_commands: Res<CosmosCommands>,

    mut structure_created: EventWriter<StructureCreated>,
    mut structure_loaded_delayed: EventWriter<SendDelayedStructureLoadEvent>,

    structure_query: Query<(Option<&Planet>, Option<&Ship>), With<Structure>>,

    all_saveable_entities: Query<Entity, With<Structure>>,
) {
    for ev in command_events.iter() {
        match ev.name.as_str() {
            "help" => {
                if ev.args.len() != 1 {
                    display_help(None, &cosmos_commands);
                } else {
                    display_help(Some(&ev.args[0]), &cosmos_commands);
                }
            }
            "ping" => {
                println!("Pong");
            }
            "list" => {
                println!("All saveable entities: ");
                for entity in all_saveable_entities.iter() {
                    print!("{} ", entity.index());
                }
                println!();
            }
            "despawn" => {
                if ev.args.len() != 1 {
                    display_help(Some("despawn"), &cosmos_commands);
                } else if let Ok(index) = ev.args[0].parse::<u64>() {
                    let entity = Entity::from_bits(index);

                    if let Some(entity_commands) = commands.get_entity(entity) {
                        entity_commands.despawn_recursive();
                        println!("Despawned entity {index}");
                    } else {
                        println!("Entity not found");
                    }
                } else {
                    println!("This must be the entity's ID (positive whole number)");
                }
            }
            "load" => {
                if ev.args.len() < 2 {
                    display_help(Some("load"), &cosmos_commands);
                } else if let Some(structure_type) = match ev.args[1].to_lowercase().as_str() {
                    "ship" => Some(StructureType::Ship),
                    "planet" => Some(StructureType::Planet),
                    _ => {
                        println!("Invalid structure type! Should be ship or planet");
                        None
                    }
                } {
                    let mut transform = Transform::default();
                    if ev.args.len() == 5 {
                        if let Ok(x) = ev.args[2].parse::<f32>() {
                            if let Ok(y) = ev.args[3].parse::<f32>() {
                                if let Ok(z) = ev.args[4].parse::<f32>() {
                                    transform.translation.x = x;
                                    transform.translation.y = y;
                                    transform.translation.z = z;
                                }
                            }
                        }
                    }

                    load_structure(
                        ev.args[0].as_str(),
                        structure_type,
                        transform,
                        &mut commands,
                        &mut structure_created,
                        &mut structure_loaded_delayed,
                    );
                }
            }
            "save" => {
                if ev.args.len() != 2 {
                    display_help(Some("save"), &cosmos_commands);
                } else if let Ok(index) = ev.args[0].parse::<u32>() {
                    if let Some(entity) = all_saveable_entities
                        .iter()
                        .find(|ent| ent.index() == index)
                    {
                        let mut entity_cmds = commands.get_entity(entity).unwrap();
                        if let Ok((planet, ship)) = structure_query.get(entity) {
                            if planet.is_some() {
                                entity_cmds.insert(SaveStructure {
                                    structure_type: StructureType::Planet,
                                    name: ev.args[1].clone(),
                                });
                            } else if ship.is_some() {
                                entity_cmds.insert(SaveStructure {
                                    structure_type: StructureType::Ship,
                                    name: ev.args[1].clone(),
                                });
                            } else {
                                println!("Error: No valid structure type (planet/ship) for this structure");
                            }
                        } else {
                            println!("You can only save structures!");
                        }
                    } else {
                        println!("Invalid entity index {index}");
                    }
                } else {
                    println!("The first argument must be the entity's index (positive number)");
                }
            }
            _ => {
                display_help(Some(&ev.text), &cosmos_commands);
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_startup_system(register_commands)
        .add_system(cosmos_command_listener);
}
