use bevy::prelude::{
    shape, Added, App, Assets, Commands, Entity, Mesh, PbrBundle, Query, ResMut, StandardMaterial,
};
use cosmos_core::universe::star::Star;

fn create_added_star(
    added: Query<(Entity, &Star), Added<Star>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut commands: Commands,
) {
    for (entity, star) in added.iter() {
        commands.entity(entity).insert(PbrBundle {
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
                emissive: star.color(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(create_added_star);
}
