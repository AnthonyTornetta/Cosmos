use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    faction::{
        FactionId,
        events::{FactionSwapAction, SwapToPlayerFactionEvent},
    },
    netty::{server::ServerLobby, sync::events::server_event::NettyEventReceived},
    physics::location::Location,
    prelude::{Ship, Station},
    state::GameState,
    structure::ship::pilot::Pilot,
};

fn on_swap_faction_from_player(
    mut nevr: EventReader<NettyEventReceived<SwapToPlayerFactionEvent>>,
    q_can_set_faction: Query<(), Or<(With<Station>, With<Ship>)>>,
    q_faction: Query<&FactionId, Or<(With<Station>, With<Ship>)>>,
    lobby: Res<ServerLobby>,
    q_player: Query<(&FactionId, &Pilot), With<Player>>,
    mut commands: Commands,
) {
    for ev in nevr.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        // This will fail for non-pilot players.
        let Ok((fac_id, pilot)) = q_player.get(player_ent) else {
            continue;
        };

        if ev.to_swap != pilot.entity {
            // Can only change the ship you're piloting
            continue;
        }

        match ev.action {
            FactionSwapAction::AssignToSelfFaction => {
                if !q_can_set_faction.contains(ev.to_swap) {
                    continue;
                }

                if q_faction.contains(ev.to_swap) {
                    // Needs to have no faction before we can set its faction
                    continue;
                }

                commands.entity(ev.to_swap).insert(*fac_id);
            }
            FactionSwapAction::RemoveFaction => {
                if !q_can_set_faction.contains(ev.to_swap) {
                    continue;
                }

                commands.entity(ev.to_swap).remove::<FactionId>();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        on_swap_faction_from_player
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing)),
    );
}
