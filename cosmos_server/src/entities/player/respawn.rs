//! Handles server death + respawn logic

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::{
        EntityId,
        health::{Dead, Health, HealthSet, MaxHealth},
        player::respawn::{RequestRespawnEvent, RespawnEvent},
    },
    inventory::{HeldItemStack, Inventory, itemstack::ItemStack},
    item::physical_item::PhysicalItem,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    persistence::LoadingDistance,
    physics::location::{Location, LocationPhysicsSet, SetPosition},
    prelude::BlockCoordinate,
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
    utils::quat_math::random_quat,
};
use serde::{Deserialize, Serialize};

use crate::universe::UniverseSystems;

use super::spawn_player::find_new_player_location;

#[derive(Component, Reflect, Serialize, Deserialize)]
/// A block the player has marked they want to respawn on.
///
/// This component does NOT imply the structure is still valid or that the block is still valid.
/// Both should be checked before performing the respawn.
pub struct RespawnBlock {
    block_coord: BlockCoordinate,
    structure_id: EntityId,
}

fn compute_respawn_location(universe_systems: &UniverseSystems) -> (Location, Quat) {
    find_new_player_location(universe_systems)
}

fn on_die(
    mut commands: Commands,
    mut q_player: Query<(&Location, &mut Inventory, &Children), (Added<Dead>, Without<HeldItemStack>)>,
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
) {
    for (location, mut inventory, children) in q_player.iter_mut() {
        if let Some(mut inv) = HeldItemStack::get_held_is_inventory_from_children_mut(children, &mut q_held_item)
            && let Some(held_is) = inv.remove_itemstack_at(0)
        {
            drop_itemstack(&mut commands, location, held_is);
        }

        inventory.retain_mut(|is| {
            drop_itemstack(&mut commands, location, is);
            None
        });
    }
}

fn on_respawn(
    lobby: Res<ServerLobby>,
    mut commands: Commands,
    universe_systems: Res<UniverseSystems>,
    mut q_player: Query<(Entity, &mut Health, &MaxHealth, &mut Velocity, Option<&RespawnBlock>), With<Dead>>,
    mut nevr: EventReader<NettyEventReceived<RequestRespawnEvent>>,
    mut nevw_respawn: NettyEventWriter<RespawnEvent>,
) {
    for ev in nevr.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok((entity, mut health, max_health, mut velocity, _respawn_block)) = q_player.get_mut(player_ent) else {
            continue;
        };
        *health = (*max_health).into();
        *velocity = Velocity::default();
        let (loc, rot) = compute_respawn_location(&universe_systems);
        // TODO: This should at some point be done on the server-side
        // *location = loc;
        // transform.rotation = rot;
        //
        nevw_respawn.write(
            RespawnEvent {
                rotation: rot,
                location: loc,
            },
            ev.client_id,
        );

        commands
            .entity(entity)
            .remove::<Dead>()
            .remove::<BuildMode>()
            .remove::<Pilot>()
            .remove::<HeldItemStack>()
            // .remove_parent() // not in place, since we just set their
            // absolute rotation
            .insert(SetPosition::Transform);
    }
}

fn drop_itemstack(commands: &mut Commands, location: &Location, is: ItemStack) {
    let dropped_item_entity = commands
        .spawn((
            PhysicalItem,
            *location,
            LoadingDistance::new(1, 2),
            Transform::from_rotation(random_quat(&mut rand::rng())),
            Velocity {
                linvel: Vec3::new(rand::random(), rand::random(), rand::random()),
                angvel: Vec3::ZERO,
            },
        ))
        .id();

    let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
    physical_item_inventory.set_itemstack_at(0, Some(is), commands);
    commands.entity(dropped_item_entity).insert(physical_item_inventory);
}

pub(super) fn register(app: &mut App) {
    app.register_type::<RespawnBlock>();

    app.add_systems(
        Update,
        (
            on_respawn.before(LocationPhysicsSet::DoPhysics),
            on_die.after(HealthSet::ProcessHealthChange),
        )
            .in_set(FixedUpdateSet::Main),
    );
}
