
use bevy::{ecs::component::HookContext, prelude::*};
use cosmos_core::{
    block::{
        Block,
        block_direction::ALL_BLOCK_DIRECTIONS,
        block_events::{BlockEventsSet, BlockInteractEvent},
        blocks::AIR_BLOCK_ID,
        data::BlockData,
    },
    events::{
        block_events::{BlockChangedEvent, BlockDataSystemParams},
        structure::structure_event::StructureEventIterator,
    },
    prelude::{BlockCoordinate, Structure},
    registry::{Registry, identifiable::Identifiable},
};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::blocks::multiblock::{
    checker::rectangle::{RectangleLimit, RectangleMultiblockError, RectangleMultiblockValidityError, check_is_valid_multiblock_bounds},
    shipyard::{Shipyard, Shipyards},
};

fn register_shipyard_component_hooks(world: &mut World) {
    world
        .register_component_hooks::<Shipyard>()
        .on_add(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure) {
                shipyards.0.push(entity);
            } else {
                world.commands().entity(structure).insert(Shipyards(vec![entity]));
            }
        })
        .on_remove(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure)
                && let Some((idx, _)) = shipyards.0.iter().enumerate().find(|x| *x.1 == entity) {
                    shipyards.0.swap_remove(idx);
                }
        });
}

fn on_place_blocks_impacting_shipyard(
    mut evr_block_changed_event: EventReader<BlockChangedEvent>,
    // blocks: Res<Registry<Block>>,
    mut q_shipyards: Query<(&Shipyards, &mut Structure)>,
    q_shipyard: Query<(&Shipyard, Entity)>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
    q_has_data: Query<(), With<Shipyard>>,
) {
    for (structure, bce) in evr_block_changed_event.read().group_by_structure() {
        let Ok((shipyards, mut structure)) = q_shipyards.get_mut(structure) else {
            continue;
        };

        // Any block placed in a shipyard will invalidate it
        for bce in bce {
            for (_, ent) in shipyards
                .iter()
                .flat_map(|x| q_shipyard.get(x))
                .filter(|(s, _)| s.coordinate_within(bce.block.coords()))
            {
                let Ok(block_coords) = q_block_data.get(ent) else {
                    continue;
                };
                let block_coords = block_coords.identifier.block.coords();

                structure.remove_block_data::<Shipyard>(block_coords, &mut bs_params, &mut q_block_data, &q_has_data);
            }
        }
    }
}

#[derive(Error, Debug, Clone, Copy, Serialize, Deserialize, Display)]
enum ShipyardError {
    #[display("Controller Touching too many frames ({_0}/1)")]
    ControllerTouchingTooManyFrames(#[error(not(source))] BlockCoordinate),
    #[display("Frame is not clear of obstructions at {_0}")]
    FrameNotClear(#[error(not(source))] BlockCoordinate),
    #[display("Missing frames")]
    MissingFrames,
}

fn compute_shipyard(structure: &Structure, controller: BlockCoordinate, frame_id: u16) -> Result<Shipyard, ShipyardError> {
    let mut starting_frame_block = ALL_BLOCK_DIRECTIONS.iter().flat_map(|x| {
        BlockCoordinate::try_from(controller + x.to_coordinates())
            .ok()
            .filter(|c| structure.is_within_blocks(*c) && structure.block_id_at(*c) == frame_id)
    });

    let starting_frame_coord = match (starting_frame_block.next(), starting_frame_block.next()) {
        (Some(c), None) => c,
        (Some(_), Some(c)) => return Err(ShipyardError::ControllerTouchingTooManyFrames(c)),
        (None, _) => return Err(ShipyardError::MissingFrames),
    };

    let valid = check_is_valid_multiblock_bounds(structure, starting_frame_coord, &[frame_id], 5, usize::MAX);

    let bounds = match valid {
        Err(e) => match e {
            RectangleMultiblockError::InvalidSquare(s) => {
                error!("{s:?}");
                return Err(ShipyardError::MissingFrames);
            }
            // This shouldn't ever happen, but just in case
            RectangleMultiblockError::TooBig => {
                error!("Got a toobig error code - this shouldn't happen.");
                return Err(ShipyardError::MissingFrames);
            }
            RectangleMultiblockError::TooSmall => {
                error!("Too small!");
                return Err(ShipyardError::MissingFrames);
            }
        },
        Ok(bounds) => bounds,
    };

    info!("Checking filled walls!");

    if let Some(e) = bounds.check_walls_filled(
        structure,
        &[frame_id, AIR_BLOCK_ID],
        &mut [RectangleLimit {
            block: frame_id,
            amount: bounds.perimeter() as usize,
        }],
    ) {
        match e {
            RectangleMultiblockValidityError::BrokenLimit { block: _, coordinate } => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
            RectangleMultiblockValidityError::InvalidBlock(coordinate) => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
        }
    }

    info!("Checking filled bounds!");
    if let Some(e) = bounds.check_inside_filled(structure, &[AIR_BLOCK_ID], &mut []) {
        match e {
            RectangleMultiblockValidityError::BrokenLimit { block: _, coordinate } => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
            RectangleMultiblockValidityError::InvalidBlock(coordinate) => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
        }
    }

    Ok(Shipyard { bounds, controller })
}

fn interact_with_shipyard(
    mut q_structure: Query<&mut Structure>,
    q_shipyard: Query<&Shipyard>,
    mut evr_interact: EventReader<BlockInteractEvent>,
    blocks: Res<Registry<Block>>,
    mut bs_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<Shipyard>>,
) {
    for ev in evr_interact.read() {
        let Some(b) = ev.block else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(b.structure()) else {
            error!("No shipyard structure!");
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:shipyard_controller") else {
            error!("No shipyard controller block!");
            return;
        };

        if structure.block_id_at(b.coords()) != block.id() {
            continue;
        }

        if let Some(shipyard) = structure.query_block_data(b.coords(), &q_shipyard) {
            // send event or something to open UI
            info!("Open UI!");
            continue;
        }

        let Some(frame) = blocks.from_id("cosmos:shipyard_frame") else {
            error!("No frame block!");
            return;
        };

        let shipyard = match compute_shipyard(&structure, b.coords(), frame.id()) {
            Err(e) => {
                error!("{e}");
                continue;
            }
            Ok(shipyard) => shipyard,
        };

        info!("Inserted shipyard {shipyard:?}!");

        structure.insert_block_data(b.coords(), shipyard, &mut bs_params, &mut q_block_data, &q_has_data);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, register_shipyard_component_hooks).add_systems(
        FixedUpdate,
        (on_place_blocks_impacting_shipyard, interact_with_shipyard)
            .chain()
            .in_set(BlockEventsSet::ProcessEvents),
    );
}
