use std::time::Duration;

use bevy::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::NoSendEntity,
    physics::location::{Location, SECTOR_DIMENSIONS, systems::Anchor},
    prelude::{Planet, Structure},
    universe::{
        star::Star,
        warp::{WarpError, WarpTo, WarpingSet},
    },
    utils::random::random_range,
};

use crate::persistence::loading::LoadingSystemSet;

const JUMP_SEARCH_RADIUS: f32 = 10_000.0;

#[derive(Component)]
#[require(Anchor, WarpAnchorDespawnSoon)]
pub struct WarpAnchor;

fn find_good_warp_spot(
    around: Location,
    q_structures: &Query<(&Location, Has<Planet>), (Without<CheckWarpSpot>, With<Structure>)>,
    q_star: &Query<&Location, (Without<CheckWarpSpot>, With<Star>)>,
) -> Result<Location, WarpError> {
    const STAR_CLEARANCE: f32 = SECTOR_DIMENSIONS * 5.0;
    const MAX_TRIES: usize = 20;

    if q_star.iter().any(|l| l.distance_sqrd(&around) < STAR_CLEARANCE * STAR_CLEARANCE) {
        return Err(WarpError::StarTooClose);
    }

    const CLEARANCE: f32 = 1_000.0;

    let locs = q_structures
        .iter()
        .filter(|(loc, _)| loc.is_within_reasonable_range(&around) && loc.distance_sqrd(&around) < JUMP_SEARCH_RADIUS * JUMP_SEARCH_RADIUS)
        .collect::<Vec<_>>();

    if locs.iter().any(|(_, is_planet)| *is_planet) {
        return Err(WarpError::Planet);
    }

    if locs.iter().all(|(loc, _)| loc.distance_sqrd(&around) > CLEARANCE * CLEARANCE) {
        return Ok(around);
    }

    let mut check;

    for _ in 0..MAX_TRIES {
        const FUDGE_LOW: f32 = -JUMP_SEARCH_RADIUS + CLEARANCE;
        const FUDGE_HIGH: f32 = JUMP_SEARCH_RADIUS - CLEARANCE;
        check = Location::new(
            Vec3::new(
                random_range(FUDGE_LOW, FUDGE_HIGH),
                random_range(FUDGE_LOW, FUDGE_HIGH),
                random_range(FUDGE_LOW, FUDGE_HIGH),
            ),
            default(),
        ) + around;

        if locs.iter().all(|(loc, _)| loc.distance_sqrd(&check) > CLEARANCE * CLEARANCE) {
            return Ok(check);
        }
    }

    Err(WarpError::TooOccupied)
}

#[derive(Component)]
struct CheckWarpSpot(Location);

fn warp_to(mut q_warp_to: Query<(Entity, &WarpTo), Added<WarpTo>>, mut commands: Commands) {
    for (ent, warp_to) in q_warp_to.iter_mut() {
        commands.entity(ent).insert(CheckWarpSpot(warp_to.loc));
        commands.spawn((
            Anchor,
            NoSendEntity,
            WarpAnchor,
            Name::new("Warp Anchor"),
            WarpAnchorDespawnSoon(0.0),
            warp_to.loc,
        ));
    }
}

fn check_for_good_warp_spot(
    mut q_check_good_warp_spot: Query<(Entity, &mut Location, &CheckWarpSpot)>,
    mut commands: Commands,
    q_structures: Query<(&Location, Has<Planet>), (Without<CheckWarpSpot>, With<Structure>)>,
    q_stars: Query<&Location, (Without<CheckWarpSpot>, With<Star>)>,
) {
    for (ent, mut loc, check_warp_spot) in q_check_good_warp_spot.iter_mut() {
        let mut ecmds = commands.entity(ent);

        let warp_to = match find_good_warp_spot(check_warp_spot.0, &q_structures, &q_stars) {
            Ok(l) => l,
            Err(e) => {
                ecmds.remove::<CheckWarpSpot>();
                ecmds.remove::<WarpTo>();
                info!("{e:?}");
                continue;
            }
        };

        ecmds.remove::<CheckWarpSpot>().remove::<WarpTo>();
        *loc = warp_to;
    }
}

#[derive(Component, Default)]
pub struct WarpAnchorDespawnSoon(f32);

const ANCHOR_LIVE_TIME: Duration = Duration::from_secs(30);

fn despawn_warp_anchors(mut q_anchor: Query<(Entity, &mut WarpAnchorDespawnSoon)>, mut commands: Commands, time: Res<Time>) {
    for (ent, mut soon) in q_anchor.iter_mut() {
        if soon.0 > ANCHOR_LIVE_TIME.as_secs_f32() {
            commands.entity(ent).insert(NeedsDespawned);
        }
        soon.0 += time.delta_secs();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        ((
            check_for_good_warp_spot.after(LoadingSystemSet::DoneLoading),
            // We need to load everything we are warping to, so leave one frame game
            despawn_warp_anchors,
            warp_to,
        )
            .chain()
            .in_set(WarpingSet::StartWarping),),
    );
}
