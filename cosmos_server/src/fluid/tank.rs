//! Ensures connected tanks properly distribute their fluid quantities


use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet, block_face::BlockFace, block_update::BlockUpdate, data::BlockData},
    ecs::mut_events::MutMessage,
    events::block_events::{BlockChangedMessage, BlockDataChangedMessage},
    fluid::data::{BlockFluidData, FluidTankBlock, StoredFluidData},
    registry::{Registry, identifiable::Identifiable},
    structure::{
        Structure,
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
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
    commands: &mut Commands,
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

            let pos_x = coord.pos_x();
            if !all_tanks_within.contains(&pos_x) && !checking.contains(&pos_x) {
                to_check.insert(pos_x);
            }
            if let Ok(neg_x) = coord.neg_x()
                && !all_tanks_within.contains(&neg_x)
                && !checking.contains(&neg_x)
            {
                to_check.insert(neg_x);
            }
            let pos_y = coord.pos_y();
            if !all_tanks_within.contains(&pos_y) && !checking.contains(&pos_y) {
                to_check.insert(pos_y);
            }
            if let Ok(neg_y) = coord.neg_y()
                && !all_tanks_within.contains(&neg_y)
                && !checking.contains(&neg_y)
            {
                to_check.insert(neg_y);
            }
            let pos_z = coord.pos_z();
            if !all_tanks_within.contains(&pos_z) && !checking.contains(&pos_z) {
                to_check.insert(pos_z);
            }
            if let Ok(neg_z) = coord.neg_z()
                && !all_tanks_within.contains(&neg_z)
                && !checking.contains(&neg_z)
            {
                to_check.insert(neg_z);
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
        let level = match tank_rotation.face_pointing_pos_y {
            BlockFace::Top => coord.y as UnboundCoordinateType,
            BlockFace::Bottom => -(coord.y as UnboundCoordinateType),
            BlockFace::Right => coord.x as UnboundCoordinateType,
            BlockFace::Left => -(coord.x as UnboundCoordinateType),
            BlockFace::Back => coord.z as UnboundCoordinateType,
            BlockFace::Front => -(coord.z as UnboundCoordinateType),
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
                .query_block_data_mut(tank_coord, q_stored_fluids, commands)
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

struct CombinedBlockMessage {
    structure_entity: Entity,
    block: BlockCoordinate,
}

fn on_add_tank(
    blocks: Res<Registry<Block>>,
    mut q_structure: Query<&mut Structure>,
    mut ev_reader: MessageReader<BlockChangedMessage>,
    mut commands: Commands,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<BlockFluidData>>,
) {
    let Some(tank) = blocks.from_id("cosmos:tank").map(|x| x.id()) else {
        return;
    };

    for ev in ev_reader.read() {
        if ev.new_block == tank {
            let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
                continue;
            };

            let coords = ev.block.coords();
            if structure.query_block_data(coords, &q_has_data).is_some() {
                continue;
            }

            structure.insert_block_data(coords, BlockFluidData::NoFluid, &mut commands, &mut q_block_data, &q_has_data);
        }
    }
}

#[derive(Resource, Debug, Default)]
struct TanksToBalance(HashMap<Entity, HashSet<BlockCoordinate>>);

fn listen_for_changed_fluid_data(
    blocks: Res<Registry<Block>>,
    mut evr_block_data_changed: MessageReader<BlockDataChangedMessage>,
    mut evr_block_changed_event: MessageReader<MutMessage<BlockUpdate>>,
    q_structure: Query<&Structure>,
    mut to_balance: ResMut<TanksToBalance>,
) {
    let Some(tank_block) = blocks.from_id("cosmos:tank") else {
        return;
    };

    for ev in evr_block_data_changed
        .read()
        .map(|x| CombinedBlockMessage {
            block: x.block.coords(),
            structure_entity: x.block.structure(),
        })
        .chain(evr_block_changed_event.read().map(|x| {
            let x = *x.read();

            CombinedBlockMessage {
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
    mut commands: Commands,
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
                &mut commands,
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (on_add_tank, listen_for_changed_fluid_data, call_balance_tanks)
            .chain()
            .in_set(BlockMessagesSet::PostProcessMessages),
    )
    .init_resource::<TanksToBalance>();
}
