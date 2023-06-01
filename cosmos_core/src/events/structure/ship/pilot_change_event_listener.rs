use bevy::prelude::{
    App, BuildChildren, Commands, Entity, EventReader, IntoSystemConfig, OnUpdate, Parent, Quat,
    Query, States, Transform, Vec3, With, Without,
};
use bevy_rapier3d::prelude::{RigidBody, Sensor};

use crate::events::structure::change_pilot_event::ChangePilotEvent;
use crate::physics::location::Location;
use crate::structure::ship::pilot::Pilot;
use crate::structure::ship::Ship;

fn event_listener(
    mut commands: Commands,
    mut event_reader: EventReader<ChangePilotEvent>,
    ship_transform_query: Query<&Transform, With<Ship>>,
    mut pilot_transform_query: Query<&mut Transform, Without<Ship>>,
    mut location_query: Query<&mut Location>,
    pilot_query: Query<&Pilot>,
) {
    for ev in event_reader.iter() {
        // Make sure there is no other player thinking they are the pilot of this ship
        if let Ok(prev_pilot) = pilot_query.get(ev.structure_entity) {
            let ship_transform = ship_transform_query
                .get(ev.structure_entity)
                .expect("Every structure should have a transform.");

            commands
                .entity(ev.structure_entity)
                .remove_children(&[prev_pilot.entity])
                .remove::<Pilot>();

            // The pilot may have disconnected
            if let Some(mut ec) = commands.get_entity(prev_pilot.entity) {
                let mut location = location_query
                    .get_mut(prev_pilot.entity)
                    .expect("Every pilot should have a location.");
                let mut pilot_transform = pilot_transform_query
                    .get_mut(prev_pilot.entity)
                    .expect("Every pilot should have a transform.");

                let direction = ship_transform.back();

                let cur_loc = *location;

                location.set_from(&(cur_loc + (direction * 2.0 + Vec3::new(0.0, 1.5, 0.0))));
                pilot_transform.rotation = Quat::IDENTITY;

                ec.remove::<Pilot>()
                    .remove::<Parent>()
                    .remove::<Sensor>()
                    .insert(RigidBody::Dynamic);
            }
        }

        if let Some(entity) = ev.pilot_entity {
            commands
                .entity(ev.structure_entity)
                .insert(Pilot { entity })
                .add_child(entity);

            let structure_location = *location_query
                .get(ev.structure_entity)
                .expect("Every structure should have a location.");

            let mut pilot_location = location_query
                .get_mut(entity)
                .expect("Every pilot should have a location.");

            pilot_location.set_from(&(structure_location + Vec3::new(0.5, -0.25, 0.5)));

            commands
                .entity(entity)
                .insert(Pilot {
                    entity: ev.structure_entity,
                })
                .insert(Sensor)
                .insert(RigidBody::Fixed);
        } else if let Some(mut ecmds) = commands.get_entity(ev.structure_entity) {
            ecmds.remove::<Pilot>();
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

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_systems((
        verify_pilot_exists.in_set(OnUpdate(playing_state)),
        event_listener
            .in_set(OnUpdate(playing_state))
            .after(verify_pilot_exists),
    ));
}
