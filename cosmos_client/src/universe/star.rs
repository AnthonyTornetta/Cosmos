//! Contains client-side logic for stars

use std::f32::consts::PI;

use bevy::{
    math::primitives::Sphere,
    pbr::{MeshMaterial3d, NotShadowCaster},
    prelude::{
        Added, App, Assets, Commands, DirectionalLight, Entity, EulerRot, Mesh, Mesh3d, Name, OnEnter, Quat, Query, ResMut,
        StandardMaterial, Transform, Update, Vec3, With, Without,
    },
};
use cosmos_core::{physics::location::SECTOR_DIMENSIONS, state::GameState, universe::star::Star};

/// Determines how bright light is based off your distance from a star.
///
/// This is a random number I made up, but looks nice enough
const LIGHT_INTENSITY_CONSTANT: f32 = 300_000_000_000_000.0;

fn point_light_from_sun(sun: Query<&Transform, With<Star>>, mut light: Query<(&mut Transform, &mut DirectionalLight), Without<Star>>) {
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
            Name::new("Star"),
            Mesh3d(meshes.add(Sphere {
                radius: SECTOR_DIMENSIONS * 2.0,
            })),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: star.color(),
                unlit: true,
                ..Default::default()
            })),
            NotShadowCaster,
        ));
    }
}

/// There is only ever one light source for stars, it is just moved around as needed
fn create_star_light_source(mut commands: Commands) {
    commands.spawn((
        Name::new("Star Light Emitter"),
        DirectionalLight {
            illuminance: 30000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_euler(EulerRot::XYZ, -PI / 4.0, 0.1, 0.1),
            ..Default::default()
        },
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (create_added_star, point_light_from_sun))
        .add_systems(OnEnter(GameState::LoadingWorld), create_star_light_source);
}
