//! Shared systems between different structure types

use bevy::{
    prelude::{App, BuildChildren, Children, Commands, Component, Or, PostUpdate, Query, With},
    reflect::Reflect,
};

use crate::{
    ecs::NeedsDespawned,
    structure::{chunk::ChunkEntity, systems::StructureSystem},
};

use super::Structure;

pub mod build_mode;

#[derive(Component, Default, Reflect, Debug, Copy, Clone)]
/// Represents the time since the last block was broken
pub struct MeltingDown(pub f32);

#[derive(Component)]
/// Marks a child of a structure as needing to be despawned when the structure itself is despawned.
///
/// If something does not have this component and its parent ship is despawned, it will have its parent removed instead of being despawned.
pub struct DespawnWithStructure;

/// Makes sure that when the structure is despawned, only that structure is despawned and not
/// any of the things docked to it (like the player walking on it)
fn save_the_kids(
    query: Query<&Children, (With<NeedsDespawned>, With<Structure>)>,
    is_this_structure: Query<(), Or<(With<ChunkEntity>, With<StructureSystem>, With<DespawnWithStructure>)>>,
    mut commands: Commands,
) {
    for children in query.iter() {
        for child in children.iter().copied().filter(|x| !is_this_structure.contains(*x)) {
            commands.entity(child).remove_parent();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, save_the_kids).register_type::<MeltingDown>();
    build_mode::register(app);
}
