use bevy::{math::DQuat, prelude::*};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    physics::location::Location,
    prelude::{Planet, Structure},
};

#[derive(Component, Debug, Clone, Copy)]
pub(super) struct LastPlanetRotation(Option<DQuat>);

fn add_last_planet_rotation(
    mut commands: Commands,
    q_needs_last_planet_rot: Query<Entity, (With<LocalPlayer>, Or<(Without<LastPlanetRotation>, Changed<ChildOf>)>)>,
) {
    let Ok(ent) = q_needs_last_planet_rot.single() else {
        return;
    };

    commands.entity(ent).insert(LastPlanetRotation(None));
}
/// WARNING: This is duplicated from the server's `planet_rotation.rs` file.
fn within_rotation_range(planet: &Structure, planet_loc: &Location, your_loc: &Location) -> bool {
    let radius = match planet {
        Structure::Dynamic(d) => d.block_dimensions() as f32,
        _ => panic!("Planet must be a dynamic structure!"),
    };

    let max_radius = radius * 2.0;

    your_loc.is_within_reasonable_range(planet_loc) && Vec3::from(*your_loc - *planet_loc).length_squared() < max_radius * max_radius
}

pub(super) fn rotate_client_around_planets(
    q_planets: Query<(&Transform, &Location, &Structure), With<Planet>>,
    mut q_local_player: Query<
        (&mut Transform, &mut Location, &mut LastPlanetRotation),
        (Without<ChildOf>, Without<Planet>, With<LocalPlayer>),
    >,
) {
    let Ok((mut trans, mut loc, mut last_planet_rotation)) = q_local_player.single_mut() else {
        return;
    };

    for (planet_transform, planet_loc, structure) in q_planets.iter() {
        if !within_rotation_range(structure, planet_loc, &loc) {
            continue;
        }

        let lpr = *last_planet_rotation;
        let planet_rotation = planet_transform.rotation.as_dquat().normalize();
        last_planet_rotation.0 = Some(planet_rotation);

        let Some(last_planet_rot) = lpr.0 else {
            return;
        };

        let delta_rot = (planet_rotation * last_planet_rot.inverse()).normalize();
        trans.rotation = (delta_rot * trans.rotation.as_dquat()).normalize().as_quat().normalize();
        loc.rotate_around(planet_loc, delta_rot);

        return;
    }

    // If we got here, no viable planets were nearby.
    last_planet_rotation.0 = None;
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (add_last_planet_rotation, rotate_client_around_planets)
            .in_set(FixedUpdateSet::Main)
            .chain(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::structure::dynamic_structure::DynamicStructure;

    #[test]
    fn following_planet_rotation_does_not_accumulate_quaternion_scale_error() {
        for radians_per_tick in [0.000044, 0.01] {
            let mut app = App::new();
            register(&mut app);
            let initial_rotation = DQuat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2);
            let local_position = Vec3::new(15.0, 50.0, 10.0).as_dvec3();
            let local_rotation = DQuat::from_rotation_y(0.9);
            let planet = app
                .world_mut()
                .spawn((
                    Planet::new(300.0),
                    Structure::Dynamic(DynamicStructure::new(16)),
                    Location::ZERO,
                    Transform::from_rotation(initial_rotation.as_quat()),
                ))
                .id();
            let player = app
                .world_mut()
                .spawn((
                    LocalPlayer,
                    Location::ZERO + (initial_rotation * local_position).as_vec3(),
                    Transform::from_rotation((initial_rotation * local_rotation).as_quat()),
                ))
                .id();
            app.world_mut().run_schedule(FixedUpdate);

            let axis = Vec3::new(0.3, 0.7, 0.2).as_dvec3().normalize();
            for tick in 1..=3600 {
                let rotation = (DQuat::from_axis_angle(axis, radians_per_tick * tick as f64) * initial_rotation).as_quat();
                // Rotation magnitudes can drift slightly during repeated f32 multiplication.
                let scale = if tick % 2 == 0 { 1.00001 } else { 0.99999 };
                let rotation = Quat::from_array(rotation.to_array().map(|value| value * scale));
                app.world_mut().get_mut::<Transform>(planet).unwrap().rotation = rotation;
                app.world_mut().run_schedule(FixedUpdate);

                let rotation = rotation.as_dquat().normalize();
                let location = app.world().get::<Location>(player).unwrap();
                let transform = app.world().get::<Transform>(player).unwrap();
                assert!(location.local.as_dvec3().distance(rotation * local_position) < 0.002);
                assert!((transform.rotation.length() - 1.0).abs() < 1e-6);
                assert!(
                    (transform.rotation * Vec3::NEG_Z)
                        .abs_diff_eq(((rotation * local_rotation) * Vec3::NEG_Z.as_dvec3()).as_vec3(), 0.0001)
                );
            }
        }
    }
}
