//! Ensures connected tanks properly distribute their fluid quantities

use bevy::{
    app::{App, Update},
    prelude::{Entity, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, With},
    utils::HashMap,
};
use cosmos_core::{
    block::{block_events::BlockEventsSet, block_update::BlockUpdate, data::BlockData, Block},
    ecs::mut_events::MutEvent,
    events::block_events::{BlockDataChangedEvent, BlockDataSystemParams},
    fluid::data::{FluidTankBlock, StoredBlockFluid},
    registry::{identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, Structure},
};

fn balance_tanks(
    structure: &Structure,
    structure_entity: Entity,
    max_fluid_storable: u32,
    tank_block_id: u16,
    origin: BlockCoordinate,
    q_stored_fluids: &Query<&StoredBlockFluid>,
    cached_tanks: &mut HashMap<(Entity, BlockCoordinate), StoredBlockFluid>,
) {
    let mut origin_stored = structure.query_block_data(origin, q_stored_fluids).copied();

    let mut checking_fluid_id = origin_stored.map(|x| x.fluid_id);

    // flow water down
    let check_pos = origin - BlockCoordinate::Y;
    if let Ok(check_pos) = BlockCoordinate::try_from(check_pos) {
        if structure.block_id_at(check_pos) == tank_block_id {
            if !cached_tanks.contains_key(&(structure_entity, check_pos)) {
                if let Some(stored) = structure.query_block_data(check_pos, q_stored_fluids) {
                    cached_tanks.insert((structure_entity, check_pos), *stored);
                }
            }

            if let Some(stored) = cached_tanks.get_mut(&(structure_entity, check_pos)) {
                let can_store_fluid = checking_fluid_id.map(|id| id == stored.fluid_id).unwrap_or(true);
                let origin_fluid_stored = origin_stored.map(|x| x.fluid_stored).unwrap_or(0);

                if can_store_fluid && stored.fluid_stored != 0 {
                    if origin_fluid_stored + stored.fluid_stored > max_fluid_storable {
                        stored.fluid_stored = stored.fluid_stored - (max_fluid_storable - origin_fluid_stored);
                        if let Some(origin_stored) = origin_stored.as_mut() {
                            origin_stored.fluid_stored = max_fluid_storable;
                        } else {
                            origin_stored = Some(StoredBlockFluid {
                                fluid_id: stored.fluid_id,
                                fluid_stored: max_fluid_storable,
                            });
                        }

                        checking_fluid_id = Some(stored.fluid_id);
                    } else {
                        if let Some(origin_stored) = origin_stored.as_mut() {
                            origin_stored.fluid_stored += stored.fluid_stored;
                        } else {
                            origin_stored = Some(StoredBlockFluid {
                                fluid_id: stored.fluid_id,
                                fluid_stored: stored.fluid_stored,
                            });
                        }

                        checking_fluid_id = Some(stored.fluid_id);
                        stored.fluid_stored = 0;
                    }
                }
            }
        }
    }

    let mut get_tank_data = |c: BlockCoordinate| {
        if structure.block_id_at(c) != tank_block_id {
            return None;
        }

        if let Some(stored_fluid) = cached_tanks.get(&(structure_entity, c)) {
            if checking_fluid_id.map(|x| x == stored_fluid.fluid_id).unwrap_or(true) {
                checking_fluid_id = Some(stored_fluid.fluid_id);
                return Some((c, stored_fluid.fluid_stored));
            }
        }

        if let Some(stored_fluid) = structure.query_block_data(c, q_stored_fluids) {
            if checking_fluid_id.map(|x| x == stored_fluid.fluid_id).unwrap_or(true) {
                checking_fluid_id = Some(stored_fluid.fluid_id);
                Some((c, stored_fluid.fluid_stored))
            } else {
                None
            }
        } else {
            Some((c, 0))
        }
    };

    let mut total_fluid_amount = origin_stored.map(|x| x.fluid_stored).unwrap_or(0);
    let mut n_tanks = 1;

    let mut right = None;
    if let Some((c, fluid_stored)) = get_tank_data(origin + BlockCoordinate::X) {
        right = Some(c);
        total_fluid_amount += fluid_stored;
        n_tanks += 1;
    }
    let mut left = None;
    if let Some((c, fluid_stored)) = BlockCoordinate::try_from(origin - BlockCoordinate::X)
        .map(|c| get_tank_data(c))
        .ok()
        .flatten()
    {
        left = Some(c);
        total_fluid_amount += fluid_stored;
        n_tanks += 1;
    }
    let mut front = None;
    if let Some((c, fluid_stored)) = get_tank_data(origin + BlockCoordinate::Z) {
        front = Some(c);
        total_fluid_amount += fluid_stored;
        n_tanks += 1;
    }
    let mut back = None;
    if let Some((c, fluid_stored)) = BlockCoordinate::try_from(origin - BlockCoordinate::Z)
        .map(|c| get_tank_data(c))
        .ok()
        .flatten()
    {
        back = Some(c);
        total_fluid_amount += fluid_stored;
        n_tanks += 1;
    }

    let avg_fluid = total_fluid_amount / n_tanks;
    // Due to rounding, the average distributed evenly may be short.
    // This represents how much fluid is still required to be distribtued post-rounding.
    let mut missing_fluid = total_fluid_amount - avg_fluid * n_tanks;

    let mut distribute_fluid = |tank_coord: BlockCoordinate| {
        let fluid_amount = if missing_fluid == 0 {
            avg_fluid
        } else {
            missing_fluid -= 1;
            avg_fluid + 1
        };

        if let Some(checking_fluid_id) = checking_fluid_id {
            cached_tanks.insert(
                (structure_entity, tank_coord),
                StoredBlockFluid {
                    fluid_id: checking_fluid_id,
                    fluid_stored: fluid_amount,
                },
            );
        }
    };

    if let Some(right) = right {
        distribute_fluid(right);
    }
    if let Some(left) = left {
        distribute_fluid(left);
    }
    if let Some(front) = front {
        distribute_fluid(front);
    }
    if let Some(back) = back {
        distribute_fluid(back);
    }

    distribute_fluid(origin);

    // This should have been distributed evenly across all tanks. If not, somehow fluid was lost in the process, which should be impossible.
    assert_eq!(missing_fluid, 0);
}

struct CombinedBlockEvent {
    structure_entity: Entity,
    block: BlockCoordinate,
}

#[derive(Resource, Debug, Default)]
/// `listen_for_changed_fluid_data` must be split into two parts due to the BlockDataChangedEvent event listener
/// needing read in `listen_for_changed_fluid_data` and written to in `apply_cached_fluid_changes`.
///
/// This resource facilitates their communication
struct CachedFluidChanges(HashMap<(Entity, BlockCoordinate), StoredBlockFluid>);

fn listen_for_changed_fluid_data(
    blocks: Res<Registry<Block>>,
    q_stored_fluids: Query<&StoredBlockFluid>,
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    mut evr_block_changed_event: EventReader<MutEvent<BlockUpdate>>,
    q_structure: Query<&Structure>,
    fluid_tank_blocks: Res<Registry<FluidTankBlock>>,
    mut fluid_changes: ResMut<CachedFluidChanges>,
) {
    let Some(tank_block) = blocks.from_id("cosmos:tank") else {
        return;
    };
    let Some(tank_fluid_block) = fluid_tank_blocks.from_id("cosmos:tank") else {
        return;
    };

    let mut block_fluid_cache = HashMap::default();

    for ev in evr_block_data_changed
        .read()
        .map(|x| CombinedBlockEvent {
            block: x.block.coords(),
            structure_entity: x.structure_entity,
        })
        .chain(evr_block_changed_event.read().map(|x| {
            let x = *x.read();

            CombinedBlockEvent {
                block: x.block().coords(),
                structure_entity: x.structure_entity(),
            }
        }))
    {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let id = structure.block_id_at(ev.block);
        if id != tank_block.id() {
            continue;
        };

        balance_tanks(
            structure,
            ev.structure_entity,
            tank_fluid_block.max_capacity(),
            tank_block.id(),
            ev.block,
            &q_stored_fluids,
            &mut block_fluid_cache,
        );
    }

    fluid_changes.0 = block_fluid_cache;
}

fn apply_cached_fluid_changes(
    mut fluid_changes: ResMut<CachedFluidChanges>,
    mut q_structure: Query<&mut Structure>,
    mut bd_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<StoredBlockFluid>>,
    q_stored_fluids: Query<&StoredBlockFluid>,
) {
    for ((structure_entity, block_coord), final_tank_data) in std::mem::take(&mut fluid_changes.0) {
        let Ok(mut structure) = q_structure.get_mut(structure_entity) else {
            continue;
        };

        // Don't trigger unnecessary change detections
        if structure
            .query_block_data(block_coord, &q_stored_fluids)
            .map(|x| *x == final_tank_data)
            .unwrap_or(false)
        {
            continue;
        }

        structure.insert_block_data(block_coord, final_tank_data, &mut bd_params, &mut q_block_data, &q_has_data);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (listen_for_changed_fluid_data, apply_cached_fluid_changes)
            .chain()
            .in_set(BlockEventsSet::ProcessEvents),
    )
    .init_resource::<CachedFluidChanges>();
}
