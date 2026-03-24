use bevy::prelude::*;
use cosmos_core::faction::{Faction, FactionSettings, Factions};
use rand::seq::IteratorRandom;

use crate::universe::FactionClaimedTerritory;

use super::*;

const N_FACTIONS: u32 = 5;
const MIN_CAPITOL_DISTANCE: SystemUnit = 10;

fn create_factions(factions: &mut Factions) {
    // very original factions
    let nekro = Faction::new(
        "Nekro Virus".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(nekro);

    let arborec = Faction::new(
        "Arborec".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(arborec);

    let emirates = Faction::new(
        "Emirates of Hacan".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(emirates);

    let saar = Faction::new(
        "Clan of Saar".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(saar);

    let saar = Faction::new(
        "Clan of Saar".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(saar);

    let sol = Faction::new(
        "Federation of Sol".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(sol);
}

fn generate_galaxy_factions(
    factions: Res<Factions>,
    mut q_galaxy: Query<(&Galaxy, &mut FactionClaimedTerritory)>,
    seed: Res<ServerSeed>,
    mut mr_generate_galaxy: MessageReader<GenerateGalaxyMessage>,
) {
    let mut fac_locs: Vec<SystemCoordinate> = vec![];
    for m in mr_generate_galaxy.read() {
        let Ok((galaxy, mut territory)) = q_galaxy.get_mut(m.0) else {
            return;
        };
        // arbitrary
        let mut rng = get_rng_for_sector(&seed, &Sector::new(100, 123, 111));

        for _ in factions.iter() {
            for _ in 0..100 {
                let Some(choice) = galaxy.stars.keys().filter(|x| x.abs().max_element() == 10).choose(&mut rng) else {
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

        for (&capital, faction) in fac_locs.iter().zip(factions.iter()) {
            territory.claim(capital, faction.id());

            let mut n_territories = 30;

            let mut to_claim_territories = vec![];
            circle(capital, &mut to_claim_territories);

            while n_territories > 0 {
                let Some((idx, &chosen)) = to_claim_territories.iter().enumerate().choose(&mut rng) else {
                    break;
                };

                to_claim_territories.swap_remove(idx);
                if !territory.is_claimed(chosen) {
                    territory.claim(chosen, faction.id());
                    continue;
                }
                n_territories -= 1;
            }
        }
    }
}

fn circle(c: SystemCoordinate, vec: &mut Vec<SystemCoordinate>) {
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 && dz == 0 {
                    continue;
                }
                vec.push(c + SystemCoordinate::new(dx, dy, dz));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        GENERATE_GALAXY_SCHEDULE,
        generate_galaxy_factions.in_set(GalaxyGenerationOrder::FactionsPlacement),
    );
}
