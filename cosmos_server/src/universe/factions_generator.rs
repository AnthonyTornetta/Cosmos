use bevy::prelude::*;
use cosmos_core::{
    netty::sync::events::netty_event::EventReceiver,
    physics::location::{SYSTEM_SECTORS, Sector},
};
use rand::{Rng, seq::IteratorRandom};

use crate::{
    init::init_world::{Noise, ServerSeed},
    rng::get_rng_for_sector,
};

use super::generation::{GenerateSystemEvent, SystemItem, UniverseSystems};

struct NpcFactionDetails {
    home_sector: Sector,
}

fn generate_factions(
    mut evr_generate_system: EventReader<GenerateSystemEvent>,
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    nosie: Res<Noise>,
) {
    for ev in evr_generate_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
            continue;
        };

        let mut rng = get_rng_for_sector(&server_seed, &ev.system.negative_most_sector());

        let faction_origin = system
            .iter()
            .filter(|maybe_asteroid| matches!(maybe_asteroid.item, SystemItem::Asteroid(_)))
            .map(|asteroid| asteroid.relative_sector(ev.system))
            .choose(&mut rng)
            .unwrap_or_else(|| {
                Sector::new(
                    rng.random_range(0..SYSTEM_SECTORS as i64),
                    rng.random_range(0..SYSTEM_SECTORS as i64),
                    rng.random_range(0..SYSTEM_SECTORS as i64),
                )
            });
    }
}

pub(super) fn register(app: &mut App) {}
