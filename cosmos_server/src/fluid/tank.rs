//! Ensures connected tanks properly distribute their fluid quantities

use std::{cell::RefCell, rc::Rc};

use bevy::{
    app::{App, Update},
    log::warn,
    prelude::{Entity, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, With},
    utils::{hashbrown::HashSet, HashMap},
};
use cosmos_core::{
    block::{block_events::BlockEventsSet, block_update::BlockUpdate, data::BlockData, Block, BlockFace},
    ecs::mut_events::MutEvent,
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent, BlockDataSystemParams},
    fluid::data::{BlockFluidData, FluidTankBlock, StoredFluidData},
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
        Structure,
    },
};

fn balance_tanks(
    structure: &Structure,
    max_fluid_storable: u32,
    tank_block_id: u16,
    origin: BlockCoordinate,
    q_stored_fluids: &mut Query<&mut BlockFluidData>,
    // cached_tanks: &mut HashMap<BlockCoordinate, BlockFluidData>,
    done_tanks: &mut HashSet<BlockCoordinate>,
    bd_params: Rc<RefCell<BlockDataSystemParams>>,
) {
    let Some(origin_stored) = structure.query_block_data(origin, q_stored_fluids).copied() else {
        warn!("Called balance_tanks for block that has no fluid data.");
        return;
    };

    let tank_rotation = structure.block_rotation(origin);

    let mut checking_fluid_id = match origin_stored {
        BlockFluidData::Fluid(sf) => Some(sf.fluid_id),
        BlockFluidData::NoFluid => None,
    };

    let mut checking: HashSet<BlockCoordinate> = HashSet::default();
    checking.insert(origin);
    let mut to_check: HashSet<BlockCoordinate> = HashSet::default();
    let mut all_tanks_within: HashSet<BlockCoordinate> = HashSet::default();

    let mut total_fluid = 0;

    while !checking.is_empty() {
        for &coord in &checking {
            if structure.block_id_at(coord) != tank_block_id {
                continue;
            }

            let Some(stored_fluid) = structure.query_block_data(coord, q_stored_fluids) else {
                warn!("Tank missing fluid data.");
                continue;
            };

            if let BlockFluidData::Fluid(sf) = stored_fluid {
                if let Some(checking_fluid_id) = checking_fluid_id {
                    if checking_fluid_id != sf.fluid_id {
                        warn!("2 tanks of different fluids were placed in the same tank system. This is bad :(");
                        continue;
                    }
                } else {
                    checking_fluid_id = Some(sf.fluid_id);
                }

                total_fluid += sf.fluid_stored;
            }

            let right = coord.right();
            if !all_tanks_within.contains(&right) && !checking.contains(&right) {
                to_check.insert(right);
            }
            if let Ok(left) = coord.left() {
                if !all_tanks_within.contains(&left) && !checking.contains(&left) {
                    to_check.insert(left);
                }
            }
            let top = coord.top();
            if !all_tanks_within.contains(&top) && !checking.contains(&top) {
                to_check.insert(top);
            }
            if let Ok(bottom) = coord.bottom() {
                if !all_tanks_within.contains(&bottom) && !checking.contains(&bottom) {
                    to_check.insert(bottom);
                }
            }
            let front = coord.front();
            if !all_tanks_within.contains(&front) && !checking.contains(&front) {
                to_check.insert(front);
            }
            if let Ok(back) = coord.back() {
                if !all_tanks_within.contains(&back) && !checking.contains(&back) {
                    to_check.insert(back);
                }
            }

            all_tanks_within.insert(coord);
        }

        let capacity = to_check.capacity();
        checking = to_check;
        to_check = HashSet::with_capacity(capacity);
    }

    if all_tanks_within.is_empty() {
        return;
    }

    let mut levels: Vec<(UnboundCoordinateType, Vec<BlockCoordinate>)> = vec![];

    for &coord in &all_tanks_within {
        let level = match tank_rotation.block_up {
            BlockFace::Top => coord.y as UnboundCoordinateType,
            BlockFace::Bottom => -(coord.y as UnboundCoordinateType),
            BlockFace::Right => coord.x as UnboundCoordinateType,
            BlockFace::Left => -(coord.x as UnboundCoordinateType),
            BlockFace::Front => coord.z as UnboundCoordinateType,
            BlockFace::Back => -(coord.z as UnboundCoordinateType),
        };

        if let Some((idx, (closest, v))) = levels.iter_mut().enumerate().find(|(_, x)| x.0 >= level) {
            if level == *closest {
                v.push(coord)
            } else {
                levels.insert(idx, (level, vec![coord]));
            }
        } else {
            levels.push((level, vec![coord]));
        }
    }

    let mut fluid_left = total_fluid;

    for (_, mut coords) in levels {
        let n_tanks = coords.len() as u32;
        let fluid_per_tank = (fluid_left / n_tanks).min(max_fluid_storable);
        let mut overflow = if fluid_per_tank == max_fluid_storable {
            0
        } else {
            fluid_left - n_tanks * fluid_per_tank
        };

        // By ensuring the ordering of tanks is always the same, micro amounts of fluid won't be "swished" around,
        // causing tons of change detection for no reason and causing super-lag.
        coords.sort_by(|a, b| {
            let a = (a.x as u128 + (a.z as u128)) << CoordinateType::BITS;
            let b = (b.x as u128 + (b.z as u128)) << CoordinateType::BITS;

            a.partial_cmp(&b).expect("comparing u128s will never fail")
        });

        for tank_coord in coords {
            done_tanks.insert(tank_coord);

            let mut fluid_stored_here = fluid_per_tank;
            if overflow > 0 {
                overflow -= 1;
                fluid_stored_here += 1;
            }

            fluid_left -= fluid_stored_here;

            let new_block_fluid_data = match checking_fluid_id {
                Some(id) => BlockFluidData::Fluid(StoredFluidData {
                    fluid_id: id,
                    fluid_stored: fluid_stored_here,
                }),
                None => {
                    assert_eq!(fluid_left, 0);

                    BlockFluidData::NoFluid
                }
            };

            let mut data_here = structure
                .query_block_data_mut(tank_coord, q_stored_fluids, bd_params.clone())
                .expect("To get here, this tank had to have had fluid data.");

            if **data_here != new_block_fluid_data {
                // Don't trigger unneccesary change detection
                **data_here = new_block_fluid_data;
            }
        }

        // Overflow must be distributed between all tanks, otherwise we've lost fluid
        debug_assert_eq!(overflow, 0);
    }

    // Ensures that no fluid was lost while distributing
    assert_eq!(fluid_left, 0);
}

struct CombinedBlockEvent {
    structure_entity: Entity,
    block: BlockCoordinate,
}

fn on_add_tank(
    blocks: Res<Registry<Block>>,
    mut q_structure: Query<&mut Structure>,
    mut ev_reader: EventReader<BlockChangedEvent>,
    mut bs_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<BlockFluidData>>,
) {
    let Some(tank) = blocks.from_id("cosmos:tank").map(|x| x.id()) else {
        return;
    };

    for ev in ev_reader.read() {
        if ev.new_block == tank {
            let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
                continue;
            };

            let coords = ev.block.coords();
            if structure.query_block_data(coords, &q_has_data).is_some() {
                continue;
            }

            structure.insert_block_data(coords, BlockFluidData::NoFluid, &mut bs_params, &mut q_block_data, &q_has_data);
        }
    }
}

#[derive(Resource, Debug, Default)]
struct TanksToBalance(HashMap<Entity, HashSet<BlockCoordinate>>);

fn listen_for_changed_fluid_data(
    blocks: Res<Registry<Block>>,
    mut evr_block_data_changed: EventReader<BlockDataChangedEvent>,
    mut evr_block_changed_event: EventReader<MutEvent<BlockUpdate>>,
    q_structure: Query<&Structure>,
    mut to_balance: ResMut<TanksToBalance>,
) {
    let Some(tank_block) = blocks.from_id("cosmos:tank") else {
        return;
    };

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

        to_balance.0.entry(ev.structure_entity).or_default().insert(ev.block);
    }
}

fn call_balance_tanks(
    blocks: Res<Registry<Block>>,
    mut q_stored_fluids: Query<&mut BlockFluidData>,
    q_structure: Query<&Structure>,
    fluid_tank_blocks: Res<Registry<FluidTankBlock>>,
    bd_params: BlockDataSystemParams,
    mut to_balance: ResMut<TanksToBalance>,
) {
    if to_balance.0.is_empty() {
        return;
    }

    let Some(tank_block) = blocks.from_id("cosmos:tank") else {
        return;
    };
    let Some(tank_fluid_block) = fluid_tank_blocks.from_id("cosmos:tank") else {
        return;
    };

    let bd_params = Rc::new(RefCell::new(bd_params));

    for (s_ent, events) in std::mem::take(&mut to_balance.0) {
        let Ok(structure) = q_structure.get(s_ent) else {
            continue;
        };

        let mut done_tanks = HashSet::default();

        for coord in events {
            let id = structure.block_id_at(coord);
            if id != tank_block.id() {
                continue;
            };

            balance_tanks(
                structure,
                tank_fluid_block.max_capacity(),
                tank_block.id(),
                coord,
                &mut q_stored_fluids,
                &mut done_tanks,
                bd_params.clone(),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_add_tank, listen_for_changed_fluid_data, call_balance_tanks)
            .chain()
            .in_set(BlockEventsSet::PostProcessEvents)
            .in_set(NetworkingSystemsSet::Between),
    )
    .init_resource::<TanksToBalance>();
}
