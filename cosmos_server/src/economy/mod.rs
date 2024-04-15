//! Handles the server's economy logic

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::Changed,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Query, ResMut},
    },
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    economy::Credits,
    netty::{
        cosmos_encoder, server_reliable_messages::ServerReliableMessages, sync::server_entity_syncing::RequestedEntityEvent,
        NettyChannelServer,
    },
};

use crate::state::GameState;

fn send_economy_updates(
    mut server: ResMut<RenetServer>,
    mut request_ent_ev_reader: EventReader<RequestedEntityEvent>,
    q_credits: Query<&Credits>,
    q_changed_credits: Query<(Entity, &Credits), Changed<Credits>>,
) {
    for (entity, &credits) in q_changed_credits.iter() {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::Credits { credits, entity }),
        )
    }

    for ev in request_ent_ev_reader.read() {
        if let Ok(credits) = q_credits.get(ev.entity).copied() {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Credits {
                    credits,
                    entity: ev.entity,
                }),
            )
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, send_economy_updates.run_if(in_state(GameState::Playing)));
}
