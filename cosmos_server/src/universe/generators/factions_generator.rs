use std::fs;

use bevy::{platform::collections::HashSet, prelude::*};
use cosmos_core::{
    faction::{Faction, FactionSettings, Factions},
    physics::location::{Location, SYSTEM_SECTORS, Sector},
    state::GameState,
    utils::quat_math::random_quat,
};
use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom, SliceRandom},
};
// use uuid::Uuid;

use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    universe::{SystemItem, SystemItemNpcFaction, UniverseSystems},
};

use super::generation::{GenerateSystemEvent, SystemGenerationSet};

// #[derive(Debug, Clone, Copy)]
// struct NpcFactionId(Uuid);
//
// impl NpcFactionId {
//     pub fn new() -> Self {
//         Self(Uuid::new_v4())
//     }
// }
//
// struct NpcFactionDetails {
//     home_sector: Sector,
//     id: NpcFactionId,
// }

fn generate_factions(
    mut evr_generate_system: EventReader<GenerateSystemEvent>,
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    mut factions: ResMut<Factions>,
) {
    for ev in evr_generate_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
            continue;
        };

        let mut rng = get_rng_for_sector(&server_seed, &ev.system.negative_most_sector());

        let n_facs = rng.random_range(3..=5);

        let mut done_zones = vec![];

        for _ in 0..n_facs {
            let mut faction_origin: Option<Sector> = None;

            const N_TRIES: u32 = 9;

            for _ in 0..N_TRIES {
                let fo = system
                    .iter()
                    .filter(|maybe_asteroid| matches!(maybe_asteroid.item, SystemItem::Asteroid(_)))
                    .map(|asteroid| asteroid.location.sector)
                    .choose(&mut rng)
                    .unwrap_or_else(|| {
                        Sector::new(
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                            rng.random_range(0..SYSTEM_SECTORS as i64),
                        )
                    })
                    + ev.system.negative_most_sector();

                if !done_zones.iter().map(|&x| x - fo).any(|x: Sector| x.abs().min_element() <= 5) {
                    faction_origin = Some(fo);
                    break;
                }
            }

            let Some(faction_origin) = faction_origin else {
                continue;
            };

            done_zones.push(faction_origin);

            let fac_noun = fs::read_to_string("assets/cosmos/factions/names/nouns.txt").expect("Missing factions names file in assets");
            let fac_adj =
                fs::read_to_string("assets/cosmos/factions/names/adjectives.txt").expect("Missing factions adjectives file in assets");

            let mut faction;
            let mut fac_id;

            loop {
                let fac_name = format!(
                    "{} {}",
                    fac_adj.split("\n").choose(&mut rng).expect("No adjective entries"),
                    fac_noun.split("\n").choose(&mut rng).expect("No noun entries")
                );
                faction = Faction::new(fac_name, vec![], Default::default(), FactionSettings { ..Default::default() });
                fac_id = faction.id();

                info!("Creating new NPC faction - {faction:?}");

                if factions.add_new_faction(faction) {
                    break;
                }
            }

            system.add_item(
                Location::new(Vec3::ZERO, faction_origin),
                random_quat(&mut rng),
                SystemItem::NpcStation(SystemItemNpcFaction {
                    faction: fac_id,
                    build_type: "capitol".into(),
                }),
            );

            let mut sectors_done = HashSet::<Sector>::default();
            sectors_done.insert(faction_origin);

            let faction_size = rng.random_range(10..15);

            let mut inner_circle = vec![];
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dz == 0 && dy == 0 && dx == 0 {
                            continue;
                        }
                        inner_circle.push(Sector::new(dx, dy, dz));
                    }
                }
            }

            inner_circle.shuffle(&mut rng);

            let buildings = ["default"];
            let mut shop_done = false;

            for i in 0..faction_size {
                let spot = inner_circle.pop().expect("Not enough sectors") * (1 + (i as f32 * 3.0 / faction_size as f32) as i64);

                system.add_item(
                    Location::new(Vec3::ZERO, faction_origin + spot),
                    random_quat(&mut rng),
                    SystemItem::NpcStation(SystemItemNpcFaction {
                        faction: fac_id,
                        build_type: if shop_done {
                            buildings.choose(&mut rng).unwrap().to_string()
                        } else {
                            "shop".into()
                        },
                    }),
                );

                shop_done = true;
            }

            info!("Done creating factions.");
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        generate_factions
            .in_set(SystemGenerationSet::FactionStations)
            .run_if(in_state(GameState::Playing)),
    );
}
