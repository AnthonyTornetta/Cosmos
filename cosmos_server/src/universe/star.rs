//! Contains server-side logic for stars

use std::slice::Iter;

use bevy::{
    core::Name,
    math::Quat,
    prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, Query, Res, ResMut, Update, With},
};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{
        cosmos_encoder, server_reliable_messages::ServerReliableMessages, sync::server_entity_syncing::RequestedEntityEvent,
        system_sets::NetworkingSystemsSet, NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::location::{Location, SYSTEM_SECTORS},
    state::GameState,
    universe::star::Star,
};

use crate::persistence::{
    saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
    SerializedData,
};

use super::{
    galaxy_generation::Galaxy,
    generation::{GenerateSystemEvent, SystemGenerationSet, SystemItem, UniverseSystems},
};

fn load_stars_in_universe(
    systems: Res<UniverseSystems>,
    mut commands: Commands,
    q_players: Query<&Location, With<Player>>,
    q_stars: Query<&Location, With<Star>>,
) {
    for (_, system) in systems.iter() {
        let Some((star_location, star)) = system
            .iter()
            .flat_map(|x| match x.item {
                SystemItem::Star(s) => Some((x.location, s)),
                _ => None,
            })
            .next()
        else {
            continue;
        };

        if q_stars.iter().any(|x| x.sector() == star_location.sector()) {
            // Star already in world
            continue;
        }

        if !q_players.iter().any(|l| {
            l.is_within_reasonable_range(&star_location)
                && ((l.sector - star_location.sector).abs().max_element() as u32) < (SYSTEM_SECTORS / 2 + 1)
        }) {
            // No player near enough to star to spawn
            continue;
        }

        commands.spawn((
            star,
            star_location,
            Name::new("Star"),
            Velocity::zero(),
            LoadingDistance::new(SYSTEM_SECTORS / 2 + 1, SYSTEM_SECTORS / 2 + 1),
        ));
    }
}

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

        (TEMPERATURE_CONSTANT * (star.temperature() / distance_scaling)).max(BACKGROUND_TEMPERATURE)
    })
}

fn generate_stars(
    mut evr_generate_system: EventReader<GenerateSystemEvent>,
    mut universe_systems: ResMut<UniverseSystems>,
    q_galaxy: Query<&Galaxy>,
) {
    for ev in evr_generate_system.read() {
        let system = ev.system;

        let Ok(galaxy) = q_galaxy.get_single() else {
            continue;
        };

        let Some(star) = galaxy.star_in_system(system) else {
            continue;
        };

        let Some(universe_system) = universe_systems.system_mut(system) else {
            continue;
        };

        universe_system.add_item(star.location, Quat::IDENTITY, SystemItem::Star(star.star));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            generate_stars.in_set(SystemGenerationSet::Star),
            load_stars_in_universe.in_set(NetworkingSystemsSet::Between),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        on_request_star
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(SAVING_SCHEDULE, on_save_star.in_set(SavingSystemSet::DoSaving));
}
