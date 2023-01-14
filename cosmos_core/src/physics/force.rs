// use bevy::prelude::{App, Commands, Component, Entity, Query, Vec3, With};
// use bevy_inspector_egui::{Inspectable, RegisterInspectable};
// use bevy_rapier3d::prelude::ExternalImpulse;

// /// Treat this as an ExternalImpulse, but one that doesn't get overwritten by random stuff
// /// Don't re-add this component, just modify it
// #[derive(Component, Default, Inspectable)]
// pub struct Force {
//     pub impulse: Vec3,
//     pub torque_impulse: Vec3,
// }

// fn system(
//     mut commands: Commands,
//     mut query: Query<(Entity, &mut Force, Option<&mut ExternalImpulse>)>,
// ) {
//     for (entity, mut force, mut external_impulse) in query.iter_mut() {
//         if force.impulse.x != 0.0 || force.impulse.y != 0.0 || force.impulse.z != 0.0 {
//             if let Some(mut external_impulse) = external_impulse {
//                 force.impulse.x += external_impulse.impulse.x;
//                 force.impulse.y += external_impulse.impulse.y;
//                 force.impulse.y += external_impulse.impulse.z;

//                 force.torque_impulse.x += external_impulse.torque_impulse.x;
//                 force.torque_impulse.y += external_impulse.torque_impulse.y;
//                 force.torque_impulse.y += external_impulse.torque_impulse.z;

//                 external_impulse.impulse = Vec3::default();
//                 external_impulse.torque_impulse = Vec3::default();
//             }

//             commands.entity(entity).insert(ExternalImpulse)
//         }
//     }
// }

// pub fn register(app: &mut App) {
//     app.register_inspectable::<Force>().add_system(system);
// }
