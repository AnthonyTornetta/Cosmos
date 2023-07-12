//! Contains server-side logic for stars

use bevy::prelude::{in_state, App, CoreSet, EventReader, IntoSystemConfig, Query, ResMut, With};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    universe::star::Star,
};

use crate::{
    netty::sync::entities::RequestedEntityEvent,
    persistence::{
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    state::GameState,
};

fn on_request_star(mut event_reader: EventReader<RequestedEntityEvent>, query: Query<&Star>, mut server: ResMut<RenetServer>) {
    for ev in event_reader.iter() {
        if let Ok(star) = query.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Star {
                    entity: ev.entity,
                    star: *star,
                }),
            );
        }
    }
}

fn on_save_star(mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<Star>)>) {
    for mut data in query.iter_mut() {
        data.set_should_save(false);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(on_request_star.run_if(in_state(GameState::Playing)))
        .add_system(on_save_star.in_base_set(CoreSet::First).after(begin_saving).before(done_saving));
}
