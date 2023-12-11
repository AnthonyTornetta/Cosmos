use bevy::{
    ecs::system::ResMut,
    prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update, With},
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{block_events::BlockInteractEvent, data::BlockDataIdentifier, Block},
    entities::player::Player,
    events::structure::change_pilot_event::ChangePilotEvent,
    inventory::netty::{InventoryIdentifier, ServerInventoryMessages},
    netty::{cosmos_encoder, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    structure::{
        ship::{pilot::Pilot, Ship},
        Structure,
    },
};

use crate::state::GameState;

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
    s_query: Query<&Structure, With<Ship>>,
    pilot_query: Query<&Pilot>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
) {
    for ev in interact_events.read() {
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(ev.structure_entity) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:storage") else {
            continue;
        };

        let block_id = ev.structure_block.block_id(structure);

        if block_id == block.id() {
            server.send_message(
                player.id(),
                NettyChannelServer::Inventory,
                cosmos_encoder::serialize(&ServerInventoryMessages::OpenInventory {
                    owner: InventoryIdentifier::BlockData(BlockDataIdentifier {
                        block: ev.structure_block,
                        structure_entity: ev.structure_entity,
                    }),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, handle_block_event.run_if(in_state(GameState::Playing)));
}
