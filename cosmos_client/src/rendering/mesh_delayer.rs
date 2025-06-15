//! Used to fix performance issues of adding a ton of meshes all at once

use std::{collections::VecDeque, mem::swap};

use bevy::prelude::{App, Assets, Commands, Entity, Mesh, Mesh3d, ResMut, Resource, Update};

#[derive(Debug)]
struct DelayedMesh {
    pub mesh: Mesh,
    pub entity: Entity,
}

#[derive(Resource, Debug, Default)]
/// A way of adding many meshes to a scene without crushing performance.
///
/// Use this instead of directly inserting it to reduce FPS lag on bulk mesh changes
pub struct DelayedMeshes(VecDeque<DelayedMesh>);

impl DelayedMeshes {
    /// Signals that a mesh should be added to this entity
    ///
    /// Use this instead of directly inserting it to reduce FPS lag on bulk mesh changes
    pub fn add_mesh(&mut self, mesh: Mesh, entity: Entity) {
        if let Some(existing) = self.0.iter_mut().find(|delayed_mesh| delayed_mesh.entity == entity) {
            existing.mesh = mesh;
        } else {
            self.0.push_back(DelayedMesh { mesh, entity });
        }
    }
}

const MESHES_PER_FRAME: usize = 5;

fn add_meshes(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands, mut meshes_to_add: ResMut<DelayedMeshes>) {
    if meshes_to_add.0.is_empty() {
        return;
    }

    let mut to_clean_meshes = VecDeque::with_capacity(meshes_to_add.0.capacity());

    swap(&mut to_clean_meshes, &mut meshes_to_add.0);

    for delayed_mesh in to_clean_meshes {
        if commands.get_entity(delayed_mesh.entity).is_ok() {
            meshes_to_add.0.push_back(delayed_mesh);
        }
    }

    for _ in 0..MESHES_PER_FRAME {
        let Some(delayed_mesh) = meshes_to_add.0.pop_front() else {
            break;
        };

        // The entity was verified to exist above
        commands.entity(delayed_mesh.entity).insert(Mesh3d(meshes.add(delayed_mesh.mesh)));
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<DelayedMeshes>().add_systems(Update, add_meshes);
}
