use bevy::prelude::*;
use bevy_rapier3d::prelude::{PhysicsWorld, Velocity};
use cosmos_core::{
    physics::{
        location::Location,
        player_world::{PlayerWorld, WorldWithin},
    },
    structure::{
        ship::{ship_builder::TShipBuilder, Ship},
        Structure,
    },
};

use crate::{
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    physics::sync_transforms_and_locations,
    structure::persistence::DelayedStructureLoadEvent,
};

use super::server_ship_builder::ServerShipBuilder;

fn on_save_structure(
    mut query: Query<(&mut SerializedData, &Structure), (With<NeedsSaved>, With<Ship>)>,
) {
    for (mut s_data, structure) in query.iter_mut() {
        s_data.serialize_data("cosmos:structure", structure);
        s_data.serialize_data("cosmos:is_ship", &true);
    }
}

fn on_load_structure(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut event_writer: EventWriter<DelayedStructureLoadEvent>,
    mut commands: Commands,
) {
    for (entity, s_data) in query.iter() {
        if s_data
            .deserialize_data::<bool>("cosmos:is_ship")
            .unwrap_or(false)
        {
            if let Some(mut structure) = s_data.deserialize_data::<Structure>("cosmos:structure") {
                let loc = s_data
                    .deserialize_data("cosmos:location")
                    .expect("Every ship should have a location when saved!");

                let mut entity_cmd = commands.entity(entity);

                let vel = s_data
                    .deserialize_data("cosmos:velocity")
                    .unwrap_or(Velocity::zero());

                let builder = ServerShipBuilder::default();

                builder.insert_ship(&mut entity_cmd, loc, vel, &mut structure);

                let entity = entity_cmd.id();

                event_writer.send(DelayedStructureLoadEvent(entity));

                commands.entity(entity).insert(structure);
            }
        }
    }
}

fn fix_location(
    mut query: Query<(Entity, &mut Location), (Added<Location>, Without<PlayerWorld>)>,
    player_worlds: Query<(&Location, &WorldWithin, &PhysicsWorld), With<PlayerWorld>>,
    mut commands: Commands,
    player_world_loc_query: Query<&Location, With<PlayerWorld>>,
) {
    for (entity, mut location) in query.iter_mut() {
        let mut best_distance = None;
        let mut best_world = None;
        let mut best_world_id = None;

        for (loc, ww, body_world) in player_worlds.iter() {
            let distance = location.distance_sqrd(loc);

            if best_distance.is_none() || distance < best_distance.unwrap() {
                best_distance = Some(distance);
                best_world = Some(*ww);
                best_world_id = Some(body_world.world_id);
            }
        }

        match (best_world, best_world_id) {
            (Some(world), Some(world_id)) => {
                if let Ok(loc) = player_world_loc_query.get(world.0) {
                    let transform = Transform::from_translation(location.relative_coords_to(loc));

                    location.last_transform_loc = Some(transform.translation);

                    commands.entity(entity).insert((
                        TransformBundle::from_transform(transform),
                        world,
                        PhysicsWorld { world_id },
                    ));
                } else {
                    warn!("A player world was missing a location");
                }
            }
            _ => {
                warn!("Something was added with a location before a player world was registered.")
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(fix_location.before(sync_transforms_and_locations))
        .add_system(on_save_structure.after(begin_saving).before(done_saving))
        .add_system(on_load_structure.after(begin_loading).before(done_loading));
}
