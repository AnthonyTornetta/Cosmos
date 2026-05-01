use bevy::{platform::collections::HashSet, prelude::*};
use cosmos_core::{
    faction::Factions,
    physics::location::{Location, SYSTEM_SECTORS, Sector, SystemUnit},
    state::GameState,
    universe::map::territory::FactionClaimedTerritory,
    utils::quat_math::random_quat,
};
use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom, SliceRandom},
};

use crate::{
    init::init_world::ServerSeed,
    rng::get_rng_for_sector,
    universe::{SystemItem, SystemItemNpcFaction, UniverseSystems},
};

use super::generation::{GenerateSystemMessage, SystemGenerationSet};

fn generate_factions(
    mut evr_generate_system: MessageReader<GenerateSystemMessage>,
    server_seed: Res<ServerSeed>,
    mut systems: ResMut<UniverseSystems>,
    factions: Res<Factions>,
    q_claimed_territory: Query<&FactionClaimedTerritory>,
) {
    for ev in evr_generate_system.read() {
        let Ok(claimed_territory) = q_claimed_territory.single() else {
            continue;
        };

        let Some(system) = systems.system_mut(ev.system) else {
            continue;
        };

        let Some(faction_here) = claimed_territory.get_claim(system.coordinate).and_then(|x| factions.from_id(&x)) else {
            continue;
        };

        let mut rng = get_rng_for_sector(&server_seed, &ev.system.negative_most_sector());

        let mut done_zones = vec![];

        let mut faction_origin: Option<Sector> = None;

        const N_TRIES: u32 = 20;

        for _ in 0..N_TRIES {
            let mut try_fac_origin = system
                .iter()
                .filter(|maybe_asteroid| matches!(maybe_asteroid.item, SystemItem::Asteroid(_)))
                .map(|asteroid| asteroid.location.relative_sector())
                .choose(&mut rng)
                .unwrap_or_else(|| {
                    Sector::new(
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                        rng.random_range(0..SYSTEM_SECTORS as i64),
                    )
                });

            const STAR_MIN: SystemUnit = SYSTEM_SECTORS as SystemUnit / 2 - 10;
            const STAR_MAX: SystemUnit = SYSTEM_SECTORS as SystemUnit / 2 - 10;

            let x = try_fac_origin.x().clamp(10, SYSTEM_SECTORS as SystemUnit - 10);
            let y = try_fac_origin.y().clamp(10, SYSTEM_SECTORS as SystemUnit - 10);
            let z = try_fac_origin.z().clamp(10, SYSTEM_SECTORS as SystemUnit - 10);

            try_fac_origin.set_x(x);
            try_fac_origin.set_y(y);
            try_fac_origin.set_z(z);

            if try_fac_origin.x() > STAR_MIN
                && try_fac_origin.x() < STAR_MAX
                && try_fac_origin.y() > STAR_MIN
                && try_fac_origin.y() < STAR_MAX
                && try_fac_origin.z() > STAR_MIN
                && try_fac_origin.z() < STAR_MAX
            {
                // too close to star
                continue;
            }

            let try_fac_origin_sector = try_fac_origin + ev.system.negative_most_sector();

            if !done_zones
                .iter()
                .map(|&x| x - try_fac_origin_sector)
                .any(|x: Sector| x.abs().min_element() <= 5)
            {
                faction_origin = Some(try_fac_origin_sector);
                break;
            }
        }

        let Some(faction_origin) = faction_origin else {
            continue;
        };

        if !ev.system.is_sector_within(faction_origin) {
            error!(
                "Somehow got invalid faction origin ({faction_origin:?} in system {:?})??",
                ev.system
            );
            continue;
        }

        done_zones.push(faction_origin);

        system.add_item(
            Location::new(Vec3::ZERO, faction_origin),
            random_quat(&mut rng),
            SystemItem::NpcStation(SystemItemNpcFaction {
                faction: faction_here.id(),
                build_type: "capitol".into(),
            }),
        );

        let mut sectors_done = HashSet::<Sector>::default();
        sectors_done.insert(faction_origin);

        let faction_size = rng.random_range(15..23);

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

            let sector = faction_origin + spot;
            if !system.is_within(sector) {
                continue;
            }

            system.add_item(
                Location::new(Vec3::ZERO, sector),
                random_quat(&mut rng),
                SystemItem::NpcStation(SystemItemNpcFaction {
                    faction: faction_here.id(),
                    build_type: if shop_done {
                        buildings.choose(&mut rng).unwrap().to_string()
                    } else {
                        "shop".into()
                    },
                }),
            );

            shop_done = true;
        }

        info!(
            "Done creating faction buildings in {} for {}.",
            system.coordinate,
            faction_here.name()
        );
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
