//! Handles client-related ship things

use bevy::{
    ecs::event::EventReader,
    prelude::{
        in_state, App, BuildChildrenTransformExt, Commands, Entity, IntoSystemConfigs, IntoSystemSetConfigs, Parent, Query, ResMut,
        SystemSet, Update, With, Without,
    },
};
use bevy_rapier3d::pipeline::CollisionEvent;
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    netty::{
        client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder, system_sets::NetworkingSystemsSet,
        NettyChannelClient,
    },
    physics::location::{CosmosBundleSet, Location, LocationPhysicsSet},
    state::GameState,
    structure::{
        chunk::CHUNK_DIMENSIONSF, loading::StructureLoadingSet, planet::Planet, shared::build_mode::BuildMode, ship::pilot::Pilot,
        Structure,
    },
};

pub mod client_ship_builder;
pub mod create_ship;
pub mod ship_movement;
pub mod ui;

fn respond_to_collisions(
    mut ev_reader: EventReader<CollisionEvent>,
    parent_query: Query<&Parent>,
    is_local_player: Query<(), (With<LocalPlayer>, Without<Pilot>)>,
    is_planet: Query<(), With<Planet>>,
    mut commands: Commands,
    mut renet_client: ResMut<RenetClient>,
) {
    for ev in ev_reader.read() {
        let CollisionEvent::Started(e1, e2, _) = ev else {
            continue;
        };

        let entities = if is_local_player.contains(*e1) {
            Some((*e1, *e2))
        } else if is_local_player.contains(*e2) {
            Some((*e2, *e1))
        } else {
            None
        };

        let Some((player_entity, hit)) = entities else {
            continue;
        };

        // the player would collide with the chunk entity, not the actual ship entity, so see if parent
        // of hit entity is a structure
        let Ok(hit_parent) = parent_query.get(hit) else {
            continue;
        };

        if !is_planet.contains(hit_parent.get()) {
            continue;
        }

        // At this point we have verified they hit a structure, now see if they are already a child
        // of that structure.
        let structure_hit_entity = hit_parent.get();

        let hitting_current_parent = parent_query.get(player_entity).is_ok_and(|p| p.get() == structure_hit_entity);

        // If they are a child of that structure, do nothing.
        if hitting_current_parent {
            continue;
        }

        // Otherwise, either remove your current parent (if you hit a non-ship) or become the child of the
        // different ship you touched if the ship has >= 10 blocks on it.

        // Even though these will always be seperate from the trans + loc below, the borrow checker doesn't know that

        if !parent_query.contains(player_entity) {
            continue;
        }

        // Otherwise just remove the parent if they hit a different structure
        commands.entity(player_entity).remove_parent_in_place();

        renet_client.send_message(
            NettyChannelClient::Reliable,
            cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
        );
    }
}

fn remove_parent_when_too_far(
    query: Query<(Entity, &Parent, &Location), (With<LocalPlayer>, Without<Structure>, Without<BuildMode>)>,
    q_structure: Query<(&Location, &Structure)>,
    mut commands: Commands,
    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((player_entity, parent, player_loc)) = query.get_single() {
        if let Ok((structure_loc, structure)) = q_structure.get(parent.get()) {
            if !matches!(structure, Structure::Full(_)) {
                return;
            }

            if player_loc.distance_sqrd(structure_loc).sqrt() >= CHUNK_DIMENSIONSF * 10.0 {
                commands.entity(player_entity).remove_parent_in_place();

                renet_client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
                );
            }
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// When the player's parent is changing, it should be done in this set
pub enum PlayerParentChangingSet {
    /// When the player's parent is changing, it should be done in this set
    ChangeParent,
}

pub(super) fn register(app: &mut App) {
    client_ship_builder::register(app);
    ship_movement::register(app);
    create_ship::register(app);
    ui::register(app);

    app.configure_sets(Update, PlayerParentChangingSet::ChangeParent.before(LocationPhysicsSet::DoPhysics));

    app.add_systems(
        Update,
        (respond_to_collisions, remove_parent_when_too_far)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .after(CosmosBundleSet::HandleCosmosBundles)
            .after(StructureLoadingSet::StructureLoaded)
            .in_set(PlayerParentChangingSet::ChangeParent)
            .run_if(in_state(GameState::Playing)),
    );
}
