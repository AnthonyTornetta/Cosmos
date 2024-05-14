use bevy::{
    app::App,
    ecs::{
        event::EventReader,
        system::{Query, Res},
    },
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    inventory::{held_item_slot::HeldItemSlot, Inventory},
    registry::Registry,
    structure::Structure,
};

fn on_interact_with_fluid(
    mut ev_reader: EventReader<BlockInteractEvent>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_held_item: Query<(&HeldItemSlot, &Inventory)>,
) {
    for ev in ev_reader.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = structure.block_at(ev.structure_block.coords(), &blocks);

        if !block.is_fluid() {
            continue;
        }

        let Ok((held_item, inventory)) = q_held_item.get(ev.interactor) else {
            continue;
        };

        // held_item.
    }
}

pub(super) fn register(app: &mut App) {}
