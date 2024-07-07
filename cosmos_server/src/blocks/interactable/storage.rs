use bevy::{
    ecs::system::ResMut,
    prelude::{in_state, App, EventReader, IntoSystemConfigs, Query, Res, Update},
};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{block_events::BlockInteractEvent, data::BlockDataIdentifier, Block},
    entities::player::Player,
    inventory::netty::{InventoryIdentifier, ServerInventoryMessages},
    netty::{cosmos_encoder, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

use crate::state::GameState;

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    s_query: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure_entity) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:storage") else {
            continue;
        };

        let block_id = s_block.structure_block.block_id(structure);

        if block_id == block.id() {
            server.send_message(
                player.id(),
                NettyChannelServer::Inventory,
                cosmos_encoder::serialize(&ServerInventoryMessages::OpenInventory {
                    owner: InventoryIdentifier::BlockData(BlockDataIdentifier {
                        block: s_block.structure_block,
                        structure_entity: s_block.structure_entity,
                        block_id,
                    }),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, handle_block_event.run_if(in_state(GameState::Playing)));
}
