// Ship core handler

use bevy::{
    prelude::{App, Commands, Component, EventReader, IntoSystemConfig, OnUpdate, Res, States},
    reflect::{FromReflect, Reflect},
};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
};

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

pub(crate) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_system(monitor_block_events.in_set(OnUpdate(playing_state)));
}
