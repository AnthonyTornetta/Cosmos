//! Contains client-side logic for stars

use bevy::{
    pbr::NotShadowCaster,
    prelude::{
        shape, Added, App, Assets, Commands, DirectionalLight, Entity, Mesh, PbrBundle, Query,
        ResMut, StandardMaterial, Transform, Vec3, With, Without,
    },
};
use cosmos_core::universe::star::Star;

fn point_light_from_sun(
    sun: Query<&Transform, With<Star>>,
    mut light: Query<&mut Transform, (With<DirectionalLight>, Without<Star>)>,
) {
    if let Ok(sun) = sun.get_single() {
        if let Ok(mut light) = light.get_single_mut() {
            light.look_at(-sun.translation, Vec3::Y);
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
                        radius: 5000.0,
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
