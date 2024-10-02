//! Contains server-side logic for stars

use std::{
    f32::consts::{E, TAU},
    slice::Iter,
};

use bevy::{
    core::Name,
    math::Vec3,
    prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, Query, Res, ResMut, Update, With},
};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    netty::{
        cosmos_encoder, server_reliable_messages::ServerReliableMessages, sync::server_entity_syncing::RequestedEntityEvent,
        system_sets::NetworkingSystemsSet, NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::location::{Location, Sector, SystemCoordinate, SystemUnit, SYSTEM_SECTORS},
    state::GameState,
    universe::star::{Star, MAX_TEMPERATURE, MIN_TEMPERATURE},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::{
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
};

use super::generation::{GenerateSystemEvent, SystemGenerationSet, SystemItem, UniverseSystems};

// Calculates the distance from the origin of a spiral arm given an angle.
fn spiral_function(theta: f32) -> f32 {
    E.powf(theta / 2.0)
}

// Calculates what offset must be necessary for spiral_function to output r given the angle (theta - offset).
// Update this whenever spiral_function is changed.
fn inverse_spiral_function(r: f32, theta: f32) -> f32 {
    theta - 2.0 * r.ln()
}

fn distance_from_star_spiral(x: f32, y: f32) -> f32 {
    // Number of spiral arms in the galaxy.
    let num_spirals: f32 = 8.0;

    let r: f32 = (x * x + y * y).sqrt();
    if r.abs() < 0.0001 {
        // Origin case, trig math gets messed up, but all arms are equally close anyways.
        return spiral_function(0.0);
    }
    let theta: f32 = y.atan2(x);

    let offset: f32 = inverse_spiral_function(r, theta);
    let spiral_index: f32 = (offset * num_spirals / TAU).round();
    let spiral_offset: f32 = spiral_index * TAU / num_spirals;

    (spiral_function(theta - spiral_offset) - r).abs() * (r / 4.0)
}

/// This gets the star - if there is one - in the system.
pub fn get_star_in_system(system: &SystemCoordinate, seed: &ServerSeed) -> Option<Star> {
    if system.y() != 0 {
        return None;
    }

    let bounds = 100.0;
    let max = 22.0;

    let ratio = max / bounds;

    let at_x = system.x() as f32 * ratio;
    let at_z = system.z() as f32 * ratio;

    if at_x.abs() > 1.0 || at_z.abs() > 1.0 {
        return None;
    }

    let seed_x = (at_x + max + 2.0) as u64;
    let seed_z = (at_z + max + 2.0) as u64;

    let local_seed = seed
        .wrapping_mul(seed_x)
        .wrapping_add(seed_z)
        .wrapping_mul(seed_z)
        .wrapping_sub(seed_x);

    let mut rng = ChaCha8Rng::seed_from_u64(local_seed);

    let distance = distance_from_star_spiral(at_x, at_z);

    let prob = 1.0 / (distance * distance);
    let num = rng.gen_range(0..10_000) as f32 / 10_000.0;

    if num < prob {
        // More likely to be low than high random number
        let rand = 1.0 - (1.0 - rng.gen::<f32>()).sqrt();
        let temperature = (rand * (MAX_TEMPERATURE - MIN_TEMPERATURE)) + MIN_TEMPERATURE;

        Some(Star::new(temperature))
    } else {
        None
    }
}

fn load_stars_in_universe(systems: Res<UniverseSystems>, mut commands: Commands, q_stars: Query<&Location, With<Star>>) {
    for (_, system) in systems.loaded() {
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

        /// 0.5 is the center of system
        const STAR_POS_OFFSET: f32 = 0.5;

        commands.spawn((
            star,
            star_location,
            Name::new("Star"),
            Velocity::zero(),
            LoadingDistance::new(SYSTEM_SECTORS / 2 + 1, SYSTEM_SECTORS / 2 + 1),
        ));
    }
}

fn generate_stars(
    mut evr_generate_system: EventReader<GenerateSystemEvent>,
    mut universe_systems: ResMut<UniverseSystems>,
    seed: Res<ServerSeed>,
) {
    for ev in evr_generate_system.read() {
        let system = ev.system;

        let Some(star) = get_star_in_system(&system, &seed) else {
            continue;
        };

        let Some(universe_system) = universe_systems.system_mut(system) else {
            continue;
        };

        /// 0.5 is the center of system
        const STAR_POS_OFFSET: f32 = 0.5;
        let loc = Location::new(
            Vec3::ZERO,
            Sector::new(
                ((system.x() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                ((system.y() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
                ((system.z() as f32 + STAR_POS_OFFSET) * SYSTEM_SECTORS as f32) as SystemUnit,
            ),
        );

        universe_system.add_item(loc, SystemItem::Star(star));
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

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (generate_stars.in_set(SystemGenerationSet::Star), load_stars_in_universe)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
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
