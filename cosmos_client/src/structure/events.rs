use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet},
    registry::Registry,
    structure::{Structure, block_health::events::BlockTakeDamageEvent},
};

// TODO: Do we need this?

fn take_damage_reader(
    mut structure_query: Query<&mut Structure>,
    mut event_reader: EventReader<BlockTakeDamageEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.read() {
        let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) else {
            continue;
        };

        if ev.new_health != 0.0 {
            structure.set_block_health(ev.block.coords(), ev.new_health, &blocks);
        }
    }
}
pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        take_damage_reader
            .after(BlockEventsSet::ProcessEvents)
            .run_if(resource_exists::<Registry<Block>>),
    );
}
