//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    ecs::{component::Component, query::Or},
    log::info,
    math::Quat,
    prelude::{in_state, App, Commands, Deref, DerefMut, IntoSystemConfigs, Query, Res, ResMut, Resource, Update, Vec3, With},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, Sector, SystemUnit, SECTOR_DIMENSIONS},
    structure::station::{station_builder::STATION_LOAD_DISTANCE, Station},
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::{
    init::init_world::ServerSeed,
    persistence::{
        is_sector_generated,
        loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
    },
    rng::get_rng_for_sector,
    state::GameState,
    universe::planet_spawner::is_planet_in_sector,
};

const SHOP_FREQUENCY: u32 = 2;

#[derive(Default, Resource, Deref, DerefMut)]
struct CachedSectors(HashSet<Sector>);

#[derive(Component)]
struct ShopNeedsPopulated;

fn spawn_shop(
    query: Query<&Location, Or<(With<Station>, With<ShopNeedsPopulated>)>>,
    players: Query<&Location, With<Player>>,
    server_seed: Res<ServerSeed>,
    mut cache: ResMut<CachedSectors>,
    mut commands: Commands,
) {
    let mut to_check_sectors = HashSet::new();

    for l in players.iter() {
        for dsz in -(STATION_LOAD_DISTANCE as SystemUnit)..=STATION_LOAD_DISTANCE as SystemUnit {
            for dsy in -(STATION_LOAD_DISTANCE as SystemUnit)..=STATION_LOAD_DISTANCE as SystemUnit {
                for dsx in -(STATION_LOAD_DISTANCE as SystemUnit)..=STATION_LOAD_DISTANCE as SystemUnit {
                    let sector = l.sector() + Sector::new(dsx, dsy, dsz);
                    to_check_sectors.insert(sector);
                }
            }
        }
    }

    let mut dead_sectors = HashSet::new();

    // Clear out unloaded sectors from the cache
    for sector in cache.iter() {
        if !to_check_sectors.contains(sector) {
            dead_sectors.insert(*sector);
        }
    }

    for dead_sector in dead_sectors {
        cache.remove(&dead_sector);
    }

    let mut sectors = HashSet::new();

    for sector in to_check_sectors {
        if !cache.contains(&sector) {
            sectors.insert(sector);
        }
    }

    for loc in query.iter() {
        let sector = loc.sector();
        cache.insert(sector);
        sectors.remove(&sector);
    }

    for sector in sectors {
        cache.insert(sector);

        if is_sector_generated(sector) || is_planet_in_sector(&sector, &server_seed) {
            // This sector has already been loaded, don't regenerate stuff
            continue;
        }

        let mut rng = get_rng_for_sector(&server_seed, &sector);

        const ORIGIN_SECTOR: Sector = Sector::new(25, 25, 25);

        if sector == ORIGIN_SECTOR || rng.gen_range(0..100) < SHOP_FREQUENCY {
            let multiplier = SECTOR_DIMENSIONS;
            let adder = -SECTOR_DIMENSIONS / 2.0;

            let loc = Location::new(
                if sector == ORIGIN_SECTOR {
                    Vec3::new(0.0, 1200.0, 0.0)
                } else {
                    Vec3::new(
                        rng.gen::<f32>() * multiplier + adder,
                        rng.gen::<f32>() * multiplier + adder,
                        rng.gen::<f32>() * multiplier + adder,
                    )
                },
                sector,
            );

            info!("Created blueprint load request @ {loc}");
            commands.spawn(NeedsBlueprintLoaded {
                path: "default_blueprints/shop/default.bp".into(),
                rotation: random_quat(&mut rng),
                spawn_at: loc,
            });
        }
    }
}

/// https://stackoverflow.com/questions/31600717/how-to-generate-a-random-quaternion-quickly
fn random_quat(rng: &mut ChaCha8Rng) -> Quat {
    let u = rng.gen::<f32>();
    let v = rng.gen::<f32>();
    let w = rng.gen::<f32>();

    Quat::from_xyzw(
        (1.0 - u).sqrt() * (2.0 * PI * v).sin(),
        (1.0 - u).sqrt() * (2.0 * PI * v).cos(),
        u.sqrt() * (2.0 * PI * w).sin(),
        u.sqrt() * (2.0 * PI * w).cos(),
    )
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        spawn_shop
            .before(LoadingBlueprintSystemSet::FlushPreBeginLoadingBlueprints)
            .run_if(on_timer(Duration::from_secs(1)))
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(CachedSectors::default());
}
