//! Contains client-side logic for black holes

use bevy::{
    color::palettes::css,
    light::NotShadowCaster,
    math::primitives::Sphere,
    pbr::MeshMaterial3d,
    prelude::{Added, App, Assets, Commands, Entity, Mesh, Mesh3d, Name, Query, ResMut, StandardMaterial, Update},
};
use cosmos_core::universe::black_hole::BlackHole;

fn create_added_black_hole(
    added: Query<(Entity, &BlackHole), Added<BlackHole>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut commands: Commands,
) {
    for (entity, black_hole) in added.iter() {
        commands.entity(entity).insert((
            Name::new("Black Hole"),
            Mesh3d(meshes.add(Sphere { radius: black_hole.radius })),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: css::BLACK.into(),
                unlit: true,
                double_sided: true,
                ..Default::default()
            })),
            NotShadowCaster,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, create_added_black_hole);
}
