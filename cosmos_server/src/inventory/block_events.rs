use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    block::block_events::{BlockBreakEvent, BlockEventsSet},
    ecs::sets::FixedUpdateSet,
    inventory::Inventory,
    item::physical_item::PhysicalItem,
    persistence::LoadingDistance,
    physics::location::Location,
    prelude::{Structure, StructureBlock},
    structure::block_health::events::BlockDestroyedEvent,
};

use crate::structure::block_health::BlockHealthSet;

fn process_event(
    structure_entity: Entity,
    block: StructureBlock,
    q_inventory: &Query<&Inventory>,
    q_structure: &Query<(&Location, &GlobalTransform, &Structure, &Velocity)>,
    commands: &mut Commands,
) {
    let Ok((location, g_trans, structure, velocity)) = q_structure.get(structure_entity) else {
        return;
    };

    let Some(inventory_here) = structure.query_block_data(block.coords(), q_inventory) else {
        return;
    };

    let structure_rot = Quat::from_affine3(&g_trans.affine());
    let item_spawn = *location + structure_rot * structure.block_relative_position(block.coords());
    let item_vel = velocity.linvel;

    for dropped_is in inventory_here.iter().flatten() {
        let dropped_is = dropped_is.clone();
        let dropped_item_entity = commands
            .spawn((
                PhysicalItem,
                item_spawn,
                Transform::from_rotation(structure_rot),
                Velocity {
                    linvel: item_vel
                        + Vec3::new(
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                            rand::random::<f32>() - 0.5,
                        ),
                    angvel: Vec3::ZERO,
                },
            ))
            .id();

        let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
        physical_item_inventory.set_itemstack_at(0, Some(dropped_is), commands);
        commands.entity(dropped_item_entity).insert(physical_item_inventory);
    }
}

fn monitor_block_detroy(
    q_inventory: Query<&Inventory>,
    q_structure: Query<(&Location, &GlobalTransform, &Structure, &Velocity)>,
    mut evr_block_destroy: EventReader<BlockDestroyedEvent>,
    mut commands: Commands,
) {
    for (block, structure_entity) in evr_block_destroy.read().map(|x| (x.block, x.structure_entity)) {
        process_event(structure_entity, block, &q_inventory, &q_structure, &mut commands);
    }
}

fn monitor_block_breaks(
    q_inventory: Query<&Inventory>,
    q_structure: Query<(&Location, &GlobalTransform, &Structure, &Velocity)>,
    mut evr_block_break: EventReader<BlockBreakEvent>,
    mut commands: Commands,
) {
    for (block, structure_entity) in evr_block_break.read().map(|x| (x.block, x.block.structure())) {
        process_event(structure_entity, block, &q_inventory, &q_structure, &mut commands);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            // We want to target blocks after events are sent, but before blocks are removed.
            monitor_block_breaks
                .in_set(BlockEventsSet::PreProcessEvents)
                .after(FixedUpdateSet::NettyReceive),
            // We want to target blocks after events are sent, but before blocks are removed.
            monitor_block_detroy
                .after(BlockHealthSet::SendHealthChanges)
                .before(BlockHealthSet::ProcessHealthChanges),
        ),
    );
}
