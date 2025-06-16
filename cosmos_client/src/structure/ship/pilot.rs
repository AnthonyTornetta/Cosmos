use bevy::prelude::*;
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    state::GameState,
    structure::ship::pilot::{Pilot, PilotFocused},
};

use crate::ui::ship_flight::indicators::{FocusedWaypointEntity, Indicating};

fn focus_looking_at(
    q_local_player: Query<&Pilot, With<LocalPlayer>>,
    q_focused: Query<Entity, With<FocusedWaypointEntity>>,
    q_indicating: Query<&Indicating>,
    mut commands: Commands,
) {
    let Ok(pilot) = q_local_player.single() else {
        return;
    };

    let Ok(focused) = q_focused.single() else {
        commands.entity(pilot.entity).remove::<PilotFocused>();
        return;
    };

    let Ok(indicating) = q_indicating.get(focused) else {
        commands.entity(pilot.entity).remove::<PilotFocused>();
        return;
    };

    commands.entity(pilot.entity).insert(PilotFocused(indicating.0));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        focus_looking_at
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
