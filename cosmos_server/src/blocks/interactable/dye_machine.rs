use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
        blocks::{COLOR_VALUES, COLORS},
        specific_blocks::dye_machine::{DyeBlock, OpenDyeMachine},
    },
    entities::player::Player,
    events::block_events::BlockDataSystemParams,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::Structure,
};

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    s_query: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut nevw_open_ui: NettyMessageWriter<OpenDyeMachine>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:dye_machine") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id == block.id() {
            nevw_open_ui.write(OpenDyeMachine(s_block), player.client_id());
        }
    }
}

fn dye_block(
    blocks: Res<Registry<Block>>,
    mut q_inventory: Query<&mut Inventory>,
    q_structure: Query<&Structure>,
    mut nevr_dye: EventReader<NettyMessageReceived<DyeBlock>>,
    bs_params: BlockDataSystemParams,
    items: Res<Registry<Item>>,
    mut commands: Commands,
    has_data: Res<ItemShouldHaveData>,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));

    for ev in nevr_dye.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            warn!("Tried to dye on non-valid structure!");
            continue;
        };

        let block = structure.block_at(ev.block.coords(), &blocks);
        if block.unlocalized_name() != "cosmos:dye_machine" {
            warn!("Tried to dye on non-dye block!");
            continue;
        }

        let Some(mut inv) = structure.query_block_data_mut(ev.block.coords(), &mut q_inventory, bs_params.clone()) else {
            warn!("No inventory on dye block!");
            continue;
        };

        let Some((color_idx, _)) = COLOR_VALUES.iter().enumerate().find(|(_, x)| **x == ev.color) else {
            warn!("Invalid color: {:?}", ev.color);
            continue;
        };

        let Some(is) = inv.itemstack_at(0) else {
            continue;
        };

        let current_item = items.from_numeric_id(is.item_id());
        let mut ul = current_item.unlocalized_name().to_owned();
        let mut sorted = COLORS.iter().collect::<Vec<_>>();
        // Needs to be sorted by len because the "ends_with" check needs to be able to handle
        // "light_grey" vs "grey".
        sorted.sort_by_key(|x| -(x.len() as i32));

        if ul != "cosmos:glass" {
            let Some(current_color) = sorted.iter().find(|&&c| ul.ends_with(c)) else {
                continue;
            };

            ul = ul[0..ul.len() - current_color.len()].to_owned();
        } else {
            ul = format!("{ul}_");
        }

        ul = format!("{ul}{}", COLORS[color_idx]);

        let Some(new_item) = items.from_id(&ul) else {
            error!("Missing color variant {ul}!");
            continue;
        };

        let qty = is.quantity();
        inv.take_and_remove_item(current_item, qty as usize, &mut commands);
        inv.insert_item(new_item, qty, &mut commands, &has_data);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (handle_block_event, dye_block)
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    );
}
