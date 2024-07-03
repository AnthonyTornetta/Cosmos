use bevy::{
    app::{App, Update},
    prelude::{in_state, Commands, Condition, IntoSystemConfigs, NextState, Res, ResMut},
};
use bevy_renet::renet::{DisconnectReason, RenetClient};

use crate::state::game_state::GameState;

use super::MainMenuSubState;

fn switch_to_title(mut commands: Commands, mut state: ResMut<NextState<GameState>>, client: Res<RenetClient>) {
    let reason = client.disconnect_reason();

    if reason != Some(DisconnectReason::DisconnectedByClient) {
        // We didn't trigger the disconnect, so give them the unexpected disconnect screen.
        commands.insert_resource(MainMenuSubState::Disconnect);
    }

    state.set(GameState::MainMenu);
}

fn is_client_disconnected(client: Option<Res<RenetClient>>) -> bool {
    client.map(|x| x.is_disconnected()).unwrap_or(true)
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        switch_to_title
            .run_if(in_state(GameState::Playing).or_else(in_state(GameState::Connecting)))
            .run_if(is_client_disconnected),
    );
}
