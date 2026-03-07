use bevy::prelude::*;
use cosmos_core::faction::Factions;
use rand::seq::IteratorRandom;

use crate::universe::{FactionClaimedTerritory, UniverseSystems};

use super::*;

const N_FACTIONS: u32 = 5;
const MIN_CAPITOL_DISTANCE: SystemUnit = 10;

fn generate_galaxy_factions(
    mut factions: ResMut<Factions>,
    mut q_galaxy: Query<(&Galaxy, &mut FactionClaimedTerritory)>,
    seed: Res<ServerSeed>,
    mut mr_generate_galaxy: MessageReader<GenerateGalaxyMessage>,
) {
    let mut fac_locs: Vec<SystemCoordinate> = vec![];
    for m in mr_generate_galaxy.read() {
        let Ok((galaxy, mut territory)) = q_galaxy.get(m.0) else {
            return;
        };
        // arbitrary
        let mut rng = get_rng_for_sector(&seed, &Sector::new(100, 123, 111));

        for _ in 0..N_FACTIONS {
            for _ in 0..100 {
                let Some(choice) = galaxy.stars.keys().choose(&mut rng) else {
                    error!("No stars generated - no factions will be created.");
                    return;
                };

                if fac_locs.iter().any(|x| (*x - *choice).abs().max_element() < MIN_CAPITOL_DISTANCE) {
                    continue;
                }

                fac_locs.push(*choice);
                break;
            }
        }

        info!("Placed capitals of {} factions ({fac_locs:?})", fac_locs.len());

        info!("Claiming territory...");

        for capital in fac_locs.iter() {}
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        GENERATE_GALAXY_SCHEDULE,
        generate_galaxy_factions.in_set(GalaxyGenerationOrder::StarsGeneration),
    );
}
