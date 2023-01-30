use bevy::ecs::schedule::StateData;
use bevy::prelude::{
    App, BuildChildren, Commands, EventReader, Parent, Quat, Query, SystemSet, Transform, Vec3,
    With,
};
use bevy::transform::TransformBundle;
use bevy_rapier3d::prelude::{RigidBody, Sensor};

use crate::entities::player::Player;
use crate::events::structure::change_pilot_event::ChangePilotEvent;
use crate::structure::ship::pilot::Pilot;

fn event_listener(
    mut commands: Commands,
    mut event_reader: EventReader<ChangePilotEvent>,
    transform_query: Query<&Transform>,
    pilot_query: Query<&Pilot>,
) {
    for ev in event_reader.iter() {
        // Make sure there is no other player thinking they are the pilot of this ship
        if let Ok(prev_pilot) = pilot_query.get(ev.structure_entity) {
            let mut transform = *transform_query.get(ev.structure_entity).unwrap();

            transform.translation += transform.back() * 2.0 + Vec3::new(0.5, 1.5, 0.5);

            commands
                .entity(ev.structure_entity)
                .remove_children(&[prev_pilot.entity])
                .remove::<Pilot>();

            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::ONE;

            commands
                .entity(prev_pilot.entity)
                .remove::<Pilot>()
                .remove::<Parent>()
                .remove::<Sensor>()
                .insert(RigidBody::Dynamic)
                .insert(TransformBundle::from_transform(transform));
        }

        if let Some(entity) = ev.pilot_entity {
            commands
                .entity(ev.structure_entity)
                .insert(Pilot { entity })
                .add_child(entity);

            commands
                .entity(entity)
                .insert(Pilot {
                    entity: ev.structure_entity,
                })
                .insert(Sensor)
                .insert(RigidBody::Fixed)
                .insert(TransformBundle::from_transform(Transform::from_xyz(
                    0.5, 0.5, 0.5,
                )));
        } else {
            commands.entity(ev.structure_entity).remove::<Pilot>();
        }
    }
}

fn keep_pilot_in_place(mut query: Query<&mut Transform, (With<Pilot>, With<Player>)>) {
    for mut transform in query.iter_mut() {
        // This is the block core's location
        // This should be moved to the camera system once that's added
        transform.translation.x = 0.5;
        transform.translation.y = -0.25;
        transform.translation.z = 0.5;
    }
}

pub fn register<T: StateData + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_system_set(
        SystemSet::on_update(playing_state)
            .with_system(event_listener)
            .with_system(keep_pilot_in_place),
    );
}
