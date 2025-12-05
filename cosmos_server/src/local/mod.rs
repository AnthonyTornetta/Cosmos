//! Logic for locally hosted servers (non-dedicated)
//!

use bevy::prelude::*;
use cosmos_core::{ecs::sets::FixedUpdateSet, entities::player::Player, state::GameState};

use crate::{commands::Operators, init::init_server::ServerSteamClient, server::stop::StopServerMessage};

#[derive(Resource)]
/// This resource being present indicates this server is a local server (belongs to the host)
pub struct LocalServer;

const NO_PLAYER_MAX_TIME: f32 = 5.0;

fn on_primary_player_disconnect(
    mut time_without_player: Local<f32>,
    mut found_player_yet: Local<bool>,
    mut mw_stop: MessageWriter<StopServerMessage>,
    client: Res<ServerSteamClient>,
    q_players: Query<&Player>,
    real_time: Res<Time<Real>>,
) {
    if q_players.iter().any(|p| p.client_id() == client.client().user().steam_id().raw()) {
        *found_player_yet = true;
    } else if *found_player_yet {
        // host of local server left - kill it
        info!("Host left local server - stopping.");
        mw_stop.write_default();
    } else {
        *time_without_player += real_time.delta_secs();
        // If the client doesn't join after ~5 seconds, something probably went wrong on the client-side, so stop
        // the server.
        if *time_without_player > NO_PLAYER_MAX_TIME {
            info!("Host did not join local server - stopping.");
            mw_stop.write_default();
        }
    }
}

fn make_local_player_op(mut operators: ResMut<Operators>, client: Res<ServerSteamClient>) {
    let steam_id = client.client().user().steam_id().raw();
    if !operators.is_operator(steam_id) {
        operators.add_operator(steam_id, "<local host>");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        on_primary_player_disconnect
            .in_set(FixedUpdateSet::Main)
            .run_if(resource_exists::<LocalServer>),
    )
    .add_systems(
        OnEnter(GameState::Playing),
        make_local_player_op.run_if(resource_exists::<LocalServer>),
    );
}
