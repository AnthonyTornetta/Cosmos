use std::{fs, io::ErrorKind};

use bevy::prelude::{App, Commands, Component, Entity, EventWriter, Query, Transform};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::structure::{
    events::{ChunkSetEvent, StructureCreated, StructureLoadedEvent},
    planet::planet_builder::TPlanetBuilder,
    ship::ship_builder::TShipBuilder,
    Structure,
};

use super::{
    planet::server_planet_builder::ServerPlanetBuilder,
    ship::server_ship_builder::ServerShipBuilder,
};

pub fn load_structure(
    structure_name: &str,
    structure_type: StructureType,
    spawn_at: Transform,
    commands: &mut Commands,
    event_writer: &mut EventWriter<StructureCreated>,
    structure_loaded: &mut EventWriter<StructureLoadedEvent>,
    chunk_set_event_writer: &mut EventWriter<ChunkSetEvent>,
) {
    if let Ok(structure_bin) = fs::read(format!(
        "saves/{}/{}.cstr",
        structure_type.name(),
        structure_name
    )) {
        if let Ok(mut structure) = bincode::deserialize::<Structure>(&structure_bin) {
            let mut entity_cmd = commands.spawn_empty();

            match structure_type {
                StructureType::Planet => {
                    let builder = ServerPlanetBuilder::default();

                    builder.insert_planet(&mut entity_cmd, spawn_at, &mut structure);
                }
                StructureType::Ship => {
                    let builder = ServerShipBuilder::default();

                    builder.insert_ship(
                        &mut entity_cmd,
                        spawn_at,
                        Velocity::zero(),
                        &mut structure,
                    );
                }
            }
            let entity = entity_cmd.id();

            event_writer.send(StructureCreated { entity });

            for chunk in structure.all_chunks_iter() {
                chunk_set_event_writer.send(ChunkSetEvent {
                    structure_entity: entity,
                    x: chunk.structure_x(),
                    y: chunk.structure_y(),
                    z: chunk.structure_z(),
                });
            }

            structure_loaded.send(StructureLoadedEvent {
                structure_entity: entity,
            });

            entity_cmd.insert(structure);
        } else {
            println!("Error parsing structure data -- is it a valid file?");
        }
    } else {
        println!(
            "No {} structure found with the name of {}!",
            structure_type.name(),
            structure_name
        );
    }
}

pub fn save_structure(
    structure: &Structure,
    file_name: &str,
    structure_type: StructureType,
) -> std::io::Result<()> {
    if let Err(e) = fs::create_dir("saves") {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    if let Err(e) = fs::create_dir(format!("saves/{}", StructureType::Ship.name())) {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    if let Err(e) = fs::create_dir(format!("saves/{}", StructureType::Planet.name())) {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    let serialized = bincode::serialize(structure).expect("Error serializing structure!");

    fs::write(
        format!("saves/{}/{file_name}.cstr", structure_type.name()),
        serialized,
    )?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum StructureType {
    Planet,
    Ship,
}

impl StructureType {
    pub fn name(&self) -> &'static str {
        match *self {
            Self::Planet => "planet",
            Self::Ship => "ship",
        }
    }
}

#[derive(Component)]
pub struct SaveStructure {
    pub name: String,
    pub structure_type: StructureType,
}

fn monitor_needs_saved(mut commands: Commands, query: Query<(Entity, &Structure, &SaveStructure)>) {
    for (entity, structure, save_structure_component) in query.iter() {
        match save_structure(
            structure,
            &save_structure_component.name,
            save_structure_component.structure_type,
        ) {
            Ok(_) => println!("Saved structure {}", save_structure_component.name),
            Err(e) => eprintln!(
                "Error saving structure {} {}",
                save_structure_component.name,
                e.to_string()
            ),
        }

        commands.entity(entity).remove::<SaveStructure>();
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(monitor_needs_saved);
}
