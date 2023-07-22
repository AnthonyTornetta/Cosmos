use bevy::{
    prelude::{in_state, App, Commands, Entity, EventWriter, IntoSystemConfigs, Query, Res, ResMut, Update},
    time::Time,
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    ecs::NeedsDespawned,
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
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
                structure.remove_block_at(block.coords(), &blocks, Some(&mut event_writer));
            } else {
                commands.entity(entity).insert(NeedsDespawned);

                server.broadcast_message(
                    NettyChannelServer::Reliable,
                    cosmos_encoder::serialize(&ServerReliableMessages::StructureRemove { entity }),
                );
            }
        }

        melting_down.0 += time.delta_seconds();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_melting_down.run_if(in_state(GameState::Playing)));
}
