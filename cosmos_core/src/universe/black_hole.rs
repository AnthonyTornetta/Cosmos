use bevy::prelude::*;
use bevy_rapier3d::prelude::{RigidBody, Velocity};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::sets::FixedUpdateSet,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    persistence::LoadingDistance,
    physics::location::{Location, SECTOR_DIMENSIONS},
};

#[derive(Reflect, Serialize, Deserialize, Component, Debug, Clone, Copy, PartialEq)]
#[require(Location)]
pub struct BlackHole {
    pub radius: f32,
}

impl BlackHole {
    const GRAV_ACCEL: f32 = 350.0;
    const MAX_EFFECT_DIST: f32 = 56014.0; // found in desmos - a bit after when this equation hits 0

    pub fn compute_acceleration(&self, distance_from_center: f32) -> f32 {
        let dist = (distance_from_center - self.radius).max(1.0);
        let dist_sqrd = dist * dist;

        // This unrealistic part makes it fade a bit faster at later distances, so we can
        // effectively "remove" the effect of the black hole far out, instead of it being super
        // small (but still noticable).
        let fade_factor = 10.0 * dist.powf(0.01);

        // max(0) because the fade_factor can make this go negative at far distances
        ((SECTOR_DIMENSIONS * SECTOR_DIMENSIONS) * Self::GRAV_ACCEL / dist_sqrd - fade_factor).max(0.0)
    }
}

impl IdentifiableComponent for BlackHole {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:black_hole"
    }
}

impl SyncableComponent for BlackHole {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

fn on_add_black_hole(q_black_hole: Query<Entity, Added<BlackHole>>, mut commands: Commands) {
    for ent in q_black_hole.iter() {
        commands.entity(ent).insert((Velocity::default(), LoadingDistance::infinite()));
    }
}

fn pull_towards_black_hole(
    q_black_hole: Query<(&Location, &BlackHole)>,
    mut q_pullable: Query<(&Location, &mut Velocity, &RigidBody)>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    for (b_hole_loc, b_hole) in q_black_hole.iter() {
        for (loc, mut vel, rb) in q_pullable.iter_mut() {
            if *rb != RigidBody::Dynamic || !loc.is_within(b_hole_loc, BlackHole::MAX_EFFECT_DIST) {
                continue;
            }

            vel.linvel += b_hole.compute_acceleration((*loc - *b_hole_loc).absolute_coords_f32().length()) * delta;
        }
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<BlackHole>(app);

    app.add_systems(
        FixedUpdate,
        (on_add_black_hole, pull_towards_black_hole).chain().in_set(FixedUpdateSet::Main),
    );
}
