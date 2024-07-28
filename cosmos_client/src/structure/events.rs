use bevy::prelude::{resource_exists, App, EventReader, IntoSystemConfigs, Query, Res, Update};
use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    netty::system_sets::NetworkingSystemsSet,
    registry::Registry,
    structure::{block_health::events::BlockTakeDamageEvent, Structure},
};

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
            .in_set(NetworkingSystemsSet::Between)
            .after(BlockEventsSet::ProcessEvents)
            .run_if(resource_exists::<Registry<Block>>),
    );
}
