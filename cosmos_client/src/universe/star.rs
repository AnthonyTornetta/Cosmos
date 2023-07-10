//! Contains client-side logic for stars

use bevy::{
    pbr::NotShadowCaster,
    prelude::{
        shape, Added, App, Assets, Commands, DirectionalLight, Entity, Mesh, PbrBundle, Query,
        ResMut, StandardMaterial, Transform, Vec3, With, Without,
    },
};
use cosmos_core::{physics::location::SECTOR_DIMENSIONS, universe::star::Star};

/// Determines how bright light is based off your distance from a star.
///
/// This is a random number I made up, but looks nice enough
const LIGHT_INTENSITY_CONSTANT: f32 = 3_000_000_000_000_000.0;

fn point_light_from_sun(
    sun: Query<&Transform, With<Star>>,
    mut light: Query<(&mut Transform, &mut DirectionalLight), Without<Star>>,
) {
    if let Ok((mut transform, mut light)) = light.get_single_mut() {
        if let Some(sun) = sun.iter().next() {
            transform.look_at(-sun.translation, Vec3::Y);
            let sun_dist_sqrd = sun.translation.dot(sun.translation);
            light.illuminance = LIGHT_INTENSITY_CONSTANT / sun_dist_sqrd;
        } else {
            light.illuminance = 0.0;
        }
    }
}

fn create_added_star(
    added: Query<(Entity, &Star), Added<Star>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut commands: Commands,
) {
    for (entity, star) in added.iter() {
        commands.entity(entity).insert((
            PbrBundle {
                mesh: meshes.add(
                    shape::UVSphere {
                        sectors: 256,
                        stacks: 256,
                        radius: SECTOR_DIMENSIONS * 2.0,
                    }
                    .into(),
                ),
                material: materials.add(StandardMaterial {
                    base_color: star.color(),
                    unlit: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            NotShadowCaster,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(create_added_star)
        .add_system(point_light_from_sun);
}
