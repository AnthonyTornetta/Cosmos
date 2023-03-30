use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    netty::{network_encoder, server_reliable_messages::ServerReliableMessages, NettyChannel},
    registry::Registry,
    structure::{
        ship::{core::MeltingDown, pilot::Pilot},
        Structure,
    },
};

use crate::state::GameState;

fn on_melting_down(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Structure, &mut MeltingDown)>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    time: Res<Time>,
    pilot_query: Query<&Pilot>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
    mut server: ResMut<RenetServer>,
) {
    for (entity, mut structure, mut melting_down) in query.iter_mut() {
        if pilot_query.contains(entity) {
            change_pilot_event.send(ChangePilotEvent {
                structure_entity: entity,
                pilot_entity: None,
            });
        }

        if melting_down.0 >= 1.0 {
            melting_down.0 -= 1.0;

            if let Some(block) = structure.all_blocks_iter(false).next() {
                structure.remove_block_at(
                    block.x,
                    block.y,
                    block.z,
                    &blocks,
                    Some(&mut event_writer),
                );
            } else {
                commands.entity(entity).despawn_recursive();

                server.broadcast_message(
                    NettyChannel::Reliable.id(),
                    network_encoder::serialize(&ServerReliableMessages::StructureRemove { entity }),
                );
            }
        }

        melting_down.0 += time.delta_seconds();
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(on_melting_down.in_set(OnUpdate(GameState::Playing)));
}
