use bevy::prelude::{
    in_state, Added, App, BuildChildren, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs,
    IntoSystemSetConfigs, Quat, Query, RemovedComponents, States, SystemSet, Transform, Update, Vec3, With,
};
use bevy_rapier3d::prelude::{RigidBody, Sensor};

use crate::entities::player::Player;
use crate::events::structure::change_pilot_event::ChangePilotEvent;
use crate::physics::location::{Location, LocationPhysicsSet};
use crate::structure::ship::pilot::Pilot;
use crate::structure::StructureTypeSet;

#[derive(Component, Debug)]
struct PilotStartingDelta(Vec3, Quat);

fn event_listener(
    mut commands: Commands,
    mut event_reader: EventReader<ChangePilotEvent>,
    location_query: Query<(&Location, &Transform)>,
    pilot_query: Query<&Pilot>,
) {
    for ev in event_reader.read() {
        // Make sure there is no other player thinking they are the pilot of this ship
        if let Ok(prev_pilot) = pilot_query.get(ev.structure_entity) {
            if let Some(mut ec) = commands.get_entity(ev.structure_entity) {
                ec.remove::<Pilot>();
            }

            if let Some(mut ec) = commands.get_entity(prev_pilot.entity) {
                ec.remove::<Pilot>();
            }
        }

        if let Some(entity) = ev.pilot_entity {
            let Ok((structure_loc, structure_transform)) = location_query.get(ev.structure_entity) else {
                // This structure probably wasn't loaded yet
                continue;
            };

            let (pilot_loc, pilot_transform) = location_query.get(entity).expect("Every pilot should have a location & transform");

            let delta = structure_transform
                .rotation
                .inverse()
                .mul_vec3(structure_loc.relative_coords_to(pilot_loc));

            let delta_rot = pilot_transform.rotation * structure_transform.rotation.inverse();

            commands.entity(ev.structure_entity).insert(Pilot { entity }).add_child(entity);

            commands.entity(entity).insert((
                Pilot {
                    entity: ev.structure_entity,
                },
                PilotStartingDelta(delta, delta_rot),
                RigidBody::Fixed,
                Sensor,
                Transform::from_xyz(0.5, -0.25, 0.5),
            ));
        } else if let Some(mut ecmds) = commands.get_entity(ev.structure_entity) {
            ecmds.remove::<Pilot>();
        }
    }
}

fn add_pilot(mut query: Query<&mut Transform, (Added<Pilot>, With<Player>)>) {
    // for mut trans in query.iter_mut() {
    // trans.translation = Vec3::new(0.5, -0.25, 0.5);
    // trans.rotation = Quat::IDENTITY;
    // }
}

#[derive(Debug, Event)]
struct RemoveSensorFrom(Entity, u8);

/// This is stupid. But the only actual solution to this would require a ton of work.
///
/// What happens is that the player leaves the ship & the client and server both move the player
/// to the correct spot. However, then the server receives a player position packet from the previous
/// spot and puts the player there shoving the ship. Then, the server receives an updated player
/// position packet and the player is back in the right spot.
///
/// To fix this we would need to some how set the player's position to a later game tick than
/// the next couple player packets it would receive, but that would require a decent bit of work.
/// So for now, we just delay the repositioning for quite a while on the server.
#[derive(Debug, Event)]
struct Bouncer(Entity, u8);

const BOUNCES: u8 = if cfg!(feature = "server") { 100 } else { 0 };

fn pilot_removed(
    mut commands: Commands,
    mut query: Query<(&mut Transform, &PilotStartingDelta)>,
    mut removed_pilots: RemovedComponents<Pilot>,
    mut event_writer: EventWriter<RemoveSensorFrom>,
) {
    for entity in removed_pilots.read() {
        if let Ok((mut trans, starting_delta)) = query.get_mut(entity) {
            commands.entity(entity).remove::<PilotStartingDelta>().insert(RigidBody::Dynamic);

            trans.translation = starting_delta.0;
            trans.rotation = starting_delta.1;

            event_writer.send(RemoveSensorFrom(entity, 0));
        }
    }
}

fn bouncer(mut reader: EventReader<Bouncer>, mut event_writer: EventWriter<RemoveSensorFrom>) {
    for ev in reader.read() {
        event_writer.send(RemoveSensorFrom(ev.0, ev.1 + 1));
    }
}

fn remove_sensor(mut reader: EventReader<RemoveSensorFrom>, mut event_writer: EventWriter<Bouncer>, mut commands: Commands) {
    for ev in reader.read() {
        if ev.1 >= BOUNCES {
            if let Some(mut e) = commands.get_entity(ev.0) {
                e.remove::<Sensor>();
            }
        } else {
            event_writer.send(Bouncer(ev.0, ev.1 + 1));
        }
    }
}

fn verify_pilot_exists(mut commands: Commands, query: Query<(Entity, &Pilot)>) {
    for (entity, pilot) in query.iter() {
        if commands.get_entity(pilot.entity).is_none() {
            commands.entity(entity).remove::<Pilot>();
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PilotEventSystemSet {
    ChangePilotListener,
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.configure_sets(
        Update,
        PilotEventSystemSet::ChangePilotListener
            // Normally you'd want to put parent-changing systems before this set, but this was all designed before this was a thing.
            // Perhaps in the future refactor this? To see how you should actually change parents, see the GravityWell logic.
            .after(LocationPhysicsSet::DoPhysics),
    );

    app.add_systems(
        Update,
        (
            add_pilot,
            pilot_removed,
            remove_sensor,
            bouncer,
            verify_pilot_exists,
            event_listener,
        )
            .in_set(PilotEventSystemSet::ChangePilotListener)
            .in_set(StructureTypeSet::Ship)
            .chain()
            .run_if(in_state(playing_state)),
    )
    .add_event::<RemoveSensorFrom>()
    .add_event::<Bouncer>();
}
