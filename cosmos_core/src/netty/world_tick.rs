//! Represents how many "game" ticks have occured

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};

/// The maximum amount of ticks per second the client/server will have.
const MAX_TPS: u64 = 20;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents how many "game" ticks have occured
pub struct WorldTick(u64);

fn tick(mut world_ticks: ResMut<WorldTick>) {
    world_ticks.0 += 1;
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        tick.run_if(resource_exists::<WorldTick>)
            .run_if(on_timer(Duration::from_millis(1000 / MAX_TPS))),
    );
}
