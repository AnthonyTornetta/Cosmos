//! Contains server-side logic for stars

use std::slice::Iter;

use bevy::prelude::{in_state, App, EventReader, IntoSystemConfigs, Query, ResMut, Update, With};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    physics::location::Location,
    universe::star::Star,
};

use crate::{
    netty::sync::entities::RequestedEntityEvent,
    persistence::{
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    state::GameState,
};

fn on_request_star(mut event_reader: EventReader<RequestedEntityEvent>, query: Query<&Star>, mut server: ResMut<RenetServer>) {
    for ev in event_reader.read() {
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

const BACKGROUND_TEMPERATURE: f32 = 50.0;
const TEMPERATURE_CONSTANT: f32 = 5.3e9;

/// Calculates the temperature at a given location from the nearest star
pub fn calculate_temperature_at(stars: Iter<'_, (Location, Star)>, location: &Location) -> Option<f32> {
    let mut closest_star = None;

    for (star_loc, star) in stars {
        let dist = location.distance_sqrd(star_loc);

        if closest_star.map(|(_, d)| d < dist).unwrap_or(true) {
            closest_star = Some((star, dist));
        }
    }

    closest_star.map(|(star, best_dist)| {
        let distance_scaling = best_dist / 2.0;

        let temperature = (TEMPERATURE_CONSTANT * (star.temperature() / distance_scaling)).max(BACKGROUND_TEMPERATURE);

        temperature
    })
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_request_star.run_if(in_state(GameState::Playing)))
        .add_systems(SAVING_SCHEDULE, on_save_star.in_set(SavingSystemSet::DoSaving));
}
