use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    physics::location::Location,
    prelude::{Planet, Structure},
};

#[derive(Component, Debug, Clone, Copy)]
struct LastPlanetRotation(Option<Quat>);

fn add_last_planet_rotation(
    mut commands: Commands,
    q_needs_last_planet_rot: Query<Entity, (With<LocalPlayer>, Without<LastPlanetRotation>)>,
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

fn rotate_client_around_planets(
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
        last_planet_rotation.0 = Some(planet_transform.rotation);

        let Some(last_planet_rot) = lpr.0 else {
            return;
        };

        let delta_rot = planet_transform.rotation * last_planet_rot.inverse();
        trans.rotation = delta_rot * trans.rotation;
        // trans.rotation *= delta_rot;
        let cur_loc = *loc;
        loc.set_from(&(*planet_loc + delta_rot * Vec3::from(cur_loc - *planet_loc)));

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
