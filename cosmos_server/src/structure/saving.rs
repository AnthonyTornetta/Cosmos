//! This is an old way of saving structures and should be redone to use the newer way
//!
//! Don't expand on this file too much, and try not to use the methods in here

use std::{fs, io::ErrorKind};

use bevy::prelude::{App, Commands, Component, Entity, EventReader, EventWriter, Query, With};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    netty::cosmos_encoder,
    physics::{location::Location, player_world::PlayerWorld},
    structure::{
        planet::planet_builder::TPlanetBuilder, ship::ship_builder::TShipBuilder,
        structure_iterator::ChunkIteratorResult, ChunkInitEvent, Structure,
    },
};

use super::{
    planet::server_planet_builder::ServerPlanetBuilder,
    ship::server_ship_builder::ServerShipBuilder,
};

/// Loading loads the structure instantly + creates the events at the same time
/// This can cause concurrency issues, so this allows the events to be generated 1 frame
/// later to avoid those issues.
#[derive(Debug)]
pub struct SendDelayedStructureLoadEvent(Entity);

#[derive(Debug)]
/// This is horrible please done use this.
///
/// Used to send a structure load event in the next frame
pub struct EvenMoreDelayedSLE(Entity);

fn send_actual_loaded_events_first(
    mut event_reader: EventReader<SendDelayedStructureLoadEvent>,
    mut event_writer: EventWriter<EvenMoreDelayedSLE>,
) {
    for ev in event_reader.iter() {
        event_writer.send(EvenMoreDelayedSLE(ev.0));
    }
}

fn send_actual_loaded_events(
    mut event_reader: EventReader<EvenMoreDelayedSLE>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    structure_query: Query<&Structure>,
) {
    for ev in event_reader.iter() {
        if let Ok(structure) = structure_query.get(ev.0) {
            for res in structure.all_chunks_iter(false) {
                // This will always be true because include_empty is false
                if let ChunkIteratorResult::FilledChunk {
                    position: (x, y, z),
                    chunk: _,
                } = res
                {
                    chunk_set_event_writer.send(ChunkInitEvent {
                        structure_entity: ev.0,
                        x,
                        y,
                        z,
                    });
                }
            }
        } else {
            println!("Error: structure still no exist");
        }
    }
}

/// TODO: Eventually turn this into event
pub fn load_structure(
    structure_name: &str,
    structure_type: StructureType,
    spawn_at: Location,
    player_worlds: &Query<&Location, With<PlayerWorld>>,
    commands: &mut Commands,
    structure_loaded: &mut EventWriter<SendDelayedStructureLoadEvent>,
) {
    if let Ok(structure_bin) = fs::read(format!(
        "saves/{}/{}.cstr",
        structure_type.name(),
        structure_name
    )) {
        println!("Loading structure {structure_name}...");

        if let Ok(mut structure) = cosmos_encoder::deserialize::<Structure>(&structure_bin) {
            let mut best_loc = None;
            let mut best_dist = f32::INFINITY;

            for world_loc in player_worlds.iter() {
                let dist = world_loc.distance_sqrd(&spawn_at);
                if dist < best_dist {
                    best_dist = dist;
                    best_loc = Some(world_loc);
                }
            }

            if let Some(world_location) = best_loc {
                let mut entity_cmd = commands.spawn_empty();

                match structure_type {
                    StructureType::Planet => {
                        let builder = ServerPlanetBuilder::default();

                        builder.insert_planet(
                            &mut entity_cmd,
                            spawn_at,
                            world_location,
                            &mut structure,
                        );
                    }
                    StructureType::Ship => {
                        let builder = ServerShipBuilder::default();

                        builder.insert_ship(
                            &mut entity_cmd,
                            spawn_at,
                            world_location,
                            Velocity::zero(),
                            &mut structure,
                        );
                    }
                }
                let entity = entity_cmd.id();

                entity_cmd.insert(structure);

                structure_loaded.send(SendDelayedStructureLoadEvent(entity));

                println!("Done with {structure_name}!");
            }
        } else {
            println!("Error parsing structure data for {structure_name} -- is it a valid file?");
        }
    } else {
        println!(
            "No {} structure found with the name of {}!",
            structure_type.name(),
            structure_name
        );
    }
}

/// Saves the given structure.
///
/// This is NOT how the structures are saved in the world, but rather used to get structure
/// files that can be loaded through commands.
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

    let serialized = cosmos_encoder::serialize(structure);

    fs::write(
        format!("saves/{}/{file_name}.cstr", structure_type.name()),
        serialized,
    )?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
/// The type of structure
pub enum StructureType {
    /// This is a planet structure
    Planet,
    /// This is a ship structure
    Ship,
}

impl StructureType {
    /// Gets the name for this structure type
    pub fn name(&self) -> &'static str {
        match *self {
            Self::Planet => "planet",
            Self::Ship => "ship",
        }
    }
}

#[derive(Component)]
/// Details of a saved structure
pub struct SaveStructure {
    /// The save name
    pub name: String,
    /// What type of structure it is
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
                save_structure_component.name, e
            ),
        }

        commands.entity(entity).remove::<SaveStructure>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(monitor_needs_saved)
        .add_event::<SendDelayedStructureLoadEvent>()
        .add_event::<EvenMoreDelayedSLE>()
        .add_system(send_actual_loaded_events_first)
        .add_system(send_actual_loaded_events);
}
