use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::{EntityId, player::Player},
    faction::{
        Faction, FactionId, FactionInvites, Factions,
        events::{
            FactionSwapAction, PlayerAcceptFactionInvitation, PlayerCreateFactionEvent, PlayerInviteToFactionEvent,
            PlayerLeaveFactionEvent, SwapToPlayerFactionEvent,
        },
    },
    netty::{server::ServerLobby, sync::events::server_event::NettyEventReceived},
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

fn on_create_faction(
    mut nevr_create_fac: EventReader<NettyEventReceived<PlayerCreateFactionEvent>>,
    lobby: Res<ServerLobby>,
    q_player_in_faction: Query<&EntityId, (Without<FactionId>, With<Player>)>,
    mut factions: ResMut<Factions>,
    mut commands: Commands,
) {
    for ev in nevr_create_fac.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            error!("Failed - Invalid player!");
            continue;
        };

        if ev.faction_name.len() > 30 {
            warn!("Failed - Name too long!");
            continue;
        }

        let Ok(ent_id) = q_player_in_faction.get(player) else {
            warn!("Failed - Already in faction!");
            continue;
        };

        if !factions.is_name_unique(&ev.faction_name) {
            info!("Failed - Name not unique!");
            continue;
        }

        let faction = Faction::new(ev.faction_name.clone(), vec![*ent_id], Default::default(), Default::default());
        let id = faction.id();

        info!("Creating faction {faction:?}");

        commands.entity(player).insert(id).remove::<FactionInvites>();
        factions.add_new_faction(faction);
    }
}

fn on_leave_faction(
    mut nevr_leave_faction: EventReader<NettyEventReceived<PlayerLeaveFactionEvent>>,
    lobby: Res<ServerLobby>,
    q_player_in_faction: Query<(&EntityId, &FactionId), With<Player>>,
    mut factions: ResMut<Factions>,
    mut commands: Commands,
) {
    for ev in nevr_leave_faction.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok((ent_id, fac_id)) = q_player_in_faction.get(player) else {
            continue;
        };

        if let Some(faction) = factions.from_id_mut(fac_id) {
            faction.remove_player(*ent_id);
            if faction.is_empty() {
                factions.remove_faction(fac_id);
            }
        }

        commands.entity(player).remove::<FactionId>();
    }
}

fn on_invite_player(
    mut nevr_leave_faction: EventReader<NettyEventReceived<PlayerInviteToFactionEvent>>,
    lobby: Res<ServerLobby>,
    q_player_in_faction: Query<&FactionId, With<Player>>,
    mut q_player_not_in_faction: Query<Option<&mut FactionInvites>, (With<Player>, Without<FactionId>)>,
    mut commands: Commands,
) {
    for ev in nevr_leave_faction.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok(fac_id) = q_player_in_faction.get(player) else {
            continue;
        };

        let Ok(inviting) = q_player_not_in_faction.get_mut(ev.inviting) else {
            continue;
        };

        if let Some(mut inviting) = inviting {
            inviting.add_invite(*fac_id);
        } else {
            commands.entity(ev.inviting).insert(FactionInvites::with_invite(*fac_id));
        }
    }
}

fn on_accept_invite(
    mut nevr_leave_faction: EventReader<NettyEventReceived<PlayerAcceptFactionInvitation>>,
    lobby: Res<ServerLobby>,
    mut q_player_not_in_faction: Query<(&EntityId, &mut FactionInvites), (With<Player>, Without<FactionId>)>,
    mut factions: ResMut<Factions>,
    mut commands: Commands,
) {
    for ev in nevr_leave_faction.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok((ent_id, mut invites)) = q_player_not_in_faction.get_mut(player) else {
            continue;
        };

        if !invites.contains(ev.faction_id) {
            continue;
        }

        let Some(fac) = factions.from_id_mut(&ev.faction_id) else {
            invites.remove_invite(ev.faction_id);
            continue;
        };

        fac.add_player(*ent_id);
        commands.entity(player).insert(ev.faction_id).remove::<FactionInvites>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            on_create_faction,
            on_leave_faction,
            on_invite_player,
            on_accept_invite,
            on_swap_faction_from_player,
        )
            .chain()
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing)),
    );
}
