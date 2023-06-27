use bevy::prelude::{App, EventReader, IntoSystemConfig, OnUpdate, Query, ResMut};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::apart_of_ship::ApartOfShip,
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
};

use crate::{netty::sync::entities::RequestedEntityEvent, state::GameState};

fn send_netty_info(
    mut event_reader: EventReader<RequestedEntityEvent>,
    mut server: ResMut<RenetServer>,
    is_apart_of_ship: Query<&ApartOfShip>,
) {
    for ev in event_reader.iter() {
        if let Ok(apart_of) = is_apart_of_ship.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::PlayerWalkOnShip {
                    player_entity: ev.entity,
                    ship_entity: apart_of.ship_entity,
                }),
            )
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(send_netty_info.in_set(OnUpdate(GameState::Playing)));
}
