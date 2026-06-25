use cosmos_core::faction::{Faction, FactionSettings, Factions};
use rand::seq::IteratorRandom;

use crate::universe::FactionClaimedTerritory;

use super::*;

const MIN_CAPITOL_DISTANCE: SystemUnit = 6;

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

    let sol = Faction::new(
        "Federation of Sol".into(),
        Default::default(),
        Default::default(),
        FactionSettings { ..Default::default() },
    );
    factions.add_new_faction(sol);
}

const NO_TERRITORY_FACTIONS: &[&str] = &["Pirate", "Merchant Federation"];

fn generate_galaxy_factions(
    mut factions: ResMut<Factions>,
    mut q_galaxy: Query<(&Galaxy, &mut FactionClaimedTerritory)>,
    seed: Res<ServerSeed>,
    mut mr_generate_galaxy: MessageReader<GenerateGalaxyMessage>,
) {
    for m in mr_generate_galaxy.read() {
        let Ok((galaxy, mut territory)) = q_galaxy.get_mut(m.0) else {
            return;
        };
        // arbitrary
        let mut rng = get_rng_for_sector(&seed, &Sector::new(100, 123, 111));

        let mut fac_locs: Vec<(&Faction, SystemCoordinate)> = vec![];

        create_factions(&mut factions);

        info!("{factions:?}");

        for f in factions.iter().filter(|x| !NO_TERRITORY_FACTIONS.contains(&x.name())) {
            for _ in 0..100 {
                let Some(choice) = galaxy.stars.keys().filter(|x| x.abs().max_element() == 10).choose(&mut rng) else {
                    error!("No stars generated - no factions will be created.");
                    return;
                };

                if fac_locs
                    .iter()
                    .any(|(_, x)| (*x - *choice).abs().max_element() < MIN_CAPITOL_DISTANCE)
                {
                    continue;
                }

                fac_locs.push((f, *choice));
                break;
            }
        }

        info!("Placed capitals of {} factions ({fac_locs:?})", fac_locs.len());

        info!("Claiming territory...");

        for (faction, capital) in fac_locs {
            territory.claim(capital, faction.id());

            let mut n_territories = 30;

            let mut to_claim_territories = vec![];
            circle(capital, &mut to_claim_territories, 1);
            claim_territory(&mut territory, &mut rng, faction, &mut n_territories, &mut to_claim_territories);

            circle(capital, &mut to_claim_territories, 3);
            claim_territory(&mut territory, &mut rng, faction, &mut n_territories, &mut to_claim_territories);
        }
    }
}

fn claim_territory(
    territory: &mut Mut<'_, FactionClaimedTerritory>,
    rng: &mut ChaCha8Rng,
    faction: &Faction,
    n_territories: &mut i32,
    to_claim_territories: &mut Vec<SystemCoordinate>,
) {
    let mut n_itrs = *n_territories * 5;
    while *n_territories > 0 && n_itrs > 0 {
        let Some((idx, &chosen)) = to_claim_territories.iter().enumerate().choose(rng) else {
            break;
        };

        n_itrs -= 1;

        to_claim_territories.swap_remove(idx);
        if territory.is_claimed(chosen) {
            continue;
        }

        territory.claim(chosen, faction.id());
        *n_territories -= 1;
    }
}

fn circle(c: SystemCoordinate, vec: &mut Vec<SystemCoordinate>, r: i64) {
    for dz in -r..=r {
        for dy in -r..=r {
            for dx in -r..=r {
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
