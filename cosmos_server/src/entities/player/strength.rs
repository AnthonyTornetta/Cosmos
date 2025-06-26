//! Player strength tracking
//!
//! Used to calculate difficulty of AI encounters.

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{entities::player::Player, netty::sync::IdentifiableComponent};
use serde::{Deserialize, Serialize};

use crate::persistence::{
    loading::LoadingSystemSet,
    make_persistent::{DefaultPersistentComponent, make_persistent},
};

#[derive(Component, Reflect, Debug, Clone, Copy, Default, Serialize, Deserialize)]
/// Represents how the enemies perceive your strength as a percentage between 0.0 and 100.0.
///
/// At 0.0%, the enemies will send their weakest fighters at you. At 100.0%, enemies will send
/// their most advanced fleets at you.
///
/// Killing pirates increases your stength, and dying lowers it.
pub struct PlayerStrength(pub f32);

impl IdentifiableComponent for PlayerStrength {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:player_strength"
    }
}

impl DefaultPersistentComponent for PlayerStrength {}

#[derive(Component, Reflect, Debug, Clone, Copy, Default, Serialize, Deserialize)]
/// Represents the total time a player has played on the server
///
/// Used for difficulty calculations
pub struct TotalTimePlayed {
    /// The total time (in seconds) the player has played
    pub time_sec: u64,
}

impl IdentifiableComponent for TotalTimePlayed {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:total_time_played"
    }
}

impl DefaultPersistentComponent for TotalTimePlayed {}

fn add_player_strength(mut commands: Commands, q_needs_player_strength: Query<Entity, (Added<Player>, Without<PlayerStrength>)>) {
    for ent in q_needs_player_strength.iter() {
        commands.entity(ent).insert(PlayerStrength::default());
    }
}

fn add_total_time_played(mut commands: Commands, q_needs_total_played: Query<Entity, (Added<Player>, Without<TotalTimePlayed>)>) {
    for ent in q_needs_total_played.iter() {
        commands.entity(ent).insert(TotalTimePlayed::default());
    }
}

fn advance_total_time(mut q_total_time: Query<&mut TotalTimePlayed>) {
    for mut tt in q_total_time.iter_mut() {
        tt.time_sec += 1;
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Player strength calculations should be based around  this
pub enum PlayerStrengthSystemSet {
    /// The player strength and play time is updated
    UpdatePlayerStrength,
}

pub(super) fn register(app: &mut App) {
    make_persistent::<PlayerStrength>(app);
    make_persistent::<TotalTimePlayed>(app);

    app.configure_sets(FixedUpdate, PlayerStrengthSystemSet::UpdatePlayerStrength);

    app.add_systems(
        FixedUpdate,
        (add_total_time_played, add_player_strength)
            .after(LoadingSystemSet::DoneLoading)
            .in_set(PlayerStrengthSystemSet::UpdatePlayerStrength),
    )
    .add_systems(
        FixedUpdate,
        advance_total_time
            .in_set(PlayerStrengthSystemSet::UpdatePlayerStrength)
            .run_if(on_timer(Duration::from_secs(1))),
    );

    app.register_type::<PlayerStrength>().register_type::<TotalTimePlayed>();
}
