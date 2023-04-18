use bevy::prelude::{in_state, App, EventReader, IntoSystemConfig, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannel},
    universe::star::Star,
};

use crate::{netty::sync::entities::RequestedEntityEvent, state::GameState};

fn on_request_star(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<&Star>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        if let Ok(star) = query.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannel::Reliable.id(),
                cosmos_encoder::serialize(&ServerReliableMessages::Star {
                    entity: ev.entity,
                    star: *star,
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(on_request_star.run_if(in_state(GameState::Playing)));
}
