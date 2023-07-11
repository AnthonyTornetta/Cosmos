//! Ship core handler

use bevy::{
    prelude::{
        App, BuildChildren, Children, Commands, Component, CoreSet, EventReader, IntoSystemConfig,
        OnUpdate, Or, Query, Res, States, With,
    },
    reflect::{FromReflect, Reflect},
};

use crate::{
    block::Block,
    ecs::{despawn_needed, NeedsDespawned},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{chunk::ChunkEntity, systems::StructureSystem},
};

use super::Ship;

#[derive(Component, Default, FromReflect, Reflect, Debug, Copy, Clone)]
/// Represents the time since the last block was broken
pub struct MeltingDown(pub f32);

fn monitor_block_events(
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut event_reader: EventReader<BlockChangedEvent>,
) {
    for ev in event_reader.iter() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:ship_core" {
            commands
                .entity(ev.structure_entity)
                .insert(MeltingDown::default());
        }
    }
}

/// Makes sure that when the ship is despawned, only that ship is despawned and not
/// any of the things docked to it (like the player walking on it)
fn save_the_kids(
    query: Query<&Children, (With<NeedsDespawned>, With<Ship>)>,
    is_this_structure: Query<(), Or<(With<ChunkEntity>, With<StructureSystem>)>>,
    mut commands: Commands,
) {
    for children in query.iter() {
        for child in children
            .iter()
            .copied()
            .filter(|x| !is_this_structure.contains(*x))
        {
            commands.entity(child).remove_parent();
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_system(monitor_block_events.in_set(OnUpdate(playing_state)))
        .add_system(
            save_the_kids
                .in_base_set(CoreSet::First)
                .before(despawn_needed),
        );
}
