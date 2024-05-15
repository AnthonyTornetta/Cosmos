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
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

struct FluidHolder {
    id: u16,
    unlocalized_name: String,
    item_id: u16,

    max_capacity: f32,
}

fn on_interact_with_fluid(
    mut ev_reader: EventReader<BlockInteractEvent>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_held_item: Query<(&HeldItemSlot, &Inventory)>,
    items: Res<Registry<Item>>,
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

        let Some(is) = inventory.itemstack_at(held_item.slot() as usize) else {
            continue;
        };

        let Some(item) = items.from_id("cosmos:fluid_cell") else {
            continue;
        };

        if is.item_id() != item.id() {
            continue;
        }

        // held_item.
    }
}

pub(super) fn register(app: &mut App) {}
