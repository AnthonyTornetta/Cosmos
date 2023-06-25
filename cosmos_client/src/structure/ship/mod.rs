//! Handles client-related ship things

use bevy::prelude::{
    App, BuildChildren, Commands, Entity, EventReader, Input, IntoSystemConfig, IntoSystemConfigs,
    KeyCode, MouseButton, OnUpdate, Parent, Query, Res, ResMut, Transform, With, Without,
};
use bevy_rapier3d::prelude::CollisionEvent;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    physics::location::{handle_child_syncing, Location},
    structure::{
        ship::{pilot::Pilot, Ship},
        Structure,
    },
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::{flags::LocalPlayer, mapping::NetworkMapping},
    state::game_state::GameState,
};

pub mod client_ship_builder;

pub(super) fn register(app: &mut App) {
    client_ship_builder::register(app);

    app.add_systems(
        (
            remove_self_from_ship,
            respond_to_collisions.after(handle_child_syncing),
        )
            .in_set(OnUpdate(GameState::Playing)),
    );
}

fn remove_self_from_ship(
    has_parent: Query<(Entity, &Parent), (With<LocalPlayer>, Without<Pilot>)>,
    ship_is_parent: Query<(), With<Ship>>,

    input_handler: Res<CosmosInputHandler>,
    inputs: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,

    mut commands: Commands,

    mut renet_client: ResMut<RenetClient>,
) {
    if let Ok((entity, parent)) = has_parent.get_single() {
        if ship_is_parent.contains(parent.get())
            && input_handler.check_just_pressed(CosmosInputs::LeaveShip, &inputs, &mouse)
        {
            commands.entity(entity).remove_parent();

            renet_client.send_message(
                NettyChannelClient::Reliable,
                cosmos_encoder::serialize(&ClientReliableMessages::LeaveShip),
            );
        }
    }
}

fn respond_to_collisions(
    mut ev_reader: EventReader<CollisionEvent>,
    parent_query: Query<&Parent>,
    is_local_player: Query<(), (With<LocalPlayer>, Without<Pilot>)>,
    is_structure: Query<(), With<Structure>>,
    is_ship: Query<(), With<Ship>>,
    mut trans_query: Query<(&mut Transform, &Location)>,
    mut commands: Commands,
    mut renet_client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
) {
    for ev in ev_reader.iter() {
        match ev {
            CollisionEvent::Started(e1, e2, _) => {
                if let Some((player_entity, hit)) = if is_local_player.contains(*e1) {
                    Some((*e1, *e2))
                } else if is_local_player.contains(*e2) {
                    Some((*e2, *e1))
                } else {
                    None
                } {
                    // the player would collide with the chunk entity, not the actual ship entity, so see if parent
                    // of hit entity is a structure
                    if let Ok(hit_parent) = parent_query.get(hit) {
                        if is_structure.contains(hit_parent.get()) {
                            // At this point we have verified they hit a structure, now see if they are already a child
                            // of that structure.
                            let structure_hit_entity = hit_parent.get();

                            let hitting_current_parent = parent_query
                                .get(player_entity)
                                .is_ok_and(|p| p.get() == structure_hit_entity);

                            // If they are a child of that structure, do nothing.
                            if !hitting_current_parent {
                                // Otherwise, either remove your current parent (if you hit a non-ship) or become the child of the
                                // different ship you touched.

                                if is_ship.contains(structure_hit_entity) {
                                    // if they hit a ship, make them a part of that one instead
                                    commands
                                        .entity(player_entity)
                                        .set_parent(structure_hit_entity);

                                    let (ship_trans, ship_loc) = trans_query
                                        .get(structure_hit_entity)
                                        .expect("All structures must have a transform");

                                    // Even though these will always be seperate from the trans + loc below, the borrow checker doesn't know that
                                    let (ship_trans, ship_loc) = (*ship_trans, *ship_loc);

                                    let (mut player_trans, player_loc) = trans_query
                                        .get_mut(player_entity)
                                        .expect("The player should have a transform + location");

                                    // Because the player's translation is always 0, 0, 0 we need to adjust it so the player is put into the
                                    // right spot in its parent.
                                    player_trans.translation = ship_trans
                                        .rotation
                                        .inverse()
                                        .mul_vec3((*player_loc - ship_loc).absolute_coords_f32());

                                    println!("Set player trans for {}", player_trans.translation);

                                    if let Some(server_ship_ent) =
                                        mapping.server_from_client(&structure_hit_entity)
                                    {
                                        renet_client.send_message(
                                            NettyChannelClient::Reliable,
                                            cosmos_encoder::serialize(
                                                &ClientReliableMessages::WalkOnShip {
                                                    ship_entity: server_ship_ent,
                                                },
                                            ),
                                        );
                                    }
                                } else {
                                    // Otherwise just remove the parent if they hit a different structure
                                    commands.entity(player_entity).remove_parent();

                                    renet_client.send_message(
                                        NettyChannelClient::Reliable,
                                        cosmos_encoder::serialize(
                                            &ClientReliableMessages::LeaveShip,
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
