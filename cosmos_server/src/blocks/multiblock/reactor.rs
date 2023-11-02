//! Handles the logic behind the creation of a reactor multiblock

use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update};
use cosmos_core::{
    block::{
        block_events::BlockInteractEvent,
        multiblock::reactor::{Reactor, ReactorBounds, ReactorPowerGenerationBlock, Reactors},
        Block, BlockFace,
    },
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate},
        structure_block::StructureBlock,
        Structure,
    },
};

use crate::state::GameState;

/// Represents the maximum dimensions of the reactor, including the reactor casing
const MAX_REACTOR_SIZE: CoordinateType = 11;

fn find_wall_coords(
    ub_controller_coords: UnboundBlockCoordinate,
    structure: &Structure,
    direction_a: UnboundBlockCoordinate,
    direction_b: UnboundBlockCoordinate,
    blocks: &Registry<Block>,
    valid_blocks: &[&Block],
) -> Option<(BlockCoordinate, BlockCoordinate)> {
    let mut width = 0;

    let mut found_coords = None;
    {
        let search_direction = direction_a;

        let mut check_coords = search_direction;
        for _ in 0..MAX_REACTOR_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            width += 1;

            let block_here = structure.block_at(check_here, blocks);

            if !valid_blocks.contains(&block_here) {
                found_coords = Some(
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(check_here) - search_direction)
                        .expect("This is guarenteed from previous logic to be within the structure"),
                );
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let Some(left_wall_coords) = found_coords else {
        return None;
    };

    let mut found_coords = None;
    {
        let search_direction = direction_b;

        let mut check_coords = search_direction;
        for _ in width..=MAX_REACTOR_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            let block_here = structure.block_at(check_here, blocks);

            if !valid_blocks.contains(&block_here) {
                found_coords = Some(
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(check_here) - search_direction)
                        .expect("This is guarenteed from previous logic to be within the structure"),
                );
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let Some(right_wall_coords) = found_coords else {
        return None;
    };

    Some((left_wall_coords, right_wall_coords))
}

fn check_is_valid_multiblock(structure: &Structure, controller_coords: BlockCoordinate, blocks: &Registry<Block>) -> Option<ReactorBounds> {
    let valid_blocks = [
        blocks.from_id("cosmos:reactor_casing").expect("Missing reactor casing"),
        blocks.from_id("cosmos:reactor_window").expect("Missing cosmos:reactor_window"),
        blocks
            .from_id("cosmos:reactor_controller")
            .expect("Missing cosmos:reactor_controller"),
    ];

    let direction = structure.block_rotation(controller_coords);

    let ub_controller_coords = UnboundBlockCoordinate::from(controller_coords);

    let mut found_coords = None;

    {
        let search_direction = direction.local_back().direction_coordinates();

        // Start 2 back to now allow a 2x2x2 reactor - minimum size is 3x3x3
        let mut check_coords = search_direction + search_direction;
        for _ in 0..MAX_REACTOR_SIZE - 2 {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            let block_here = structure.block_at(check_here, blocks);

            if valid_blocks.contains(&block_here) {
                found_coords = Some(check_here);
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let Some(back_wall_coords) = found_coords else {
        return None;
    };

    let Some((left_wall_coords, right_wall_coords)) = find_wall_coords(
        ub_controller_coords,
        structure,
        direction.local_left().direction_coordinates(),
        direction.local_right().direction_coordinates(),
        blocks,
        &valid_blocks,
    ) else {
        return None;
    };

    let Some((down_wall_coords, up_wall_coords)) = find_wall_coords(
        ub_controller_coords,
        structure,
        direction.local_bottom().direction_coordinates(),
        direction.local_top().direction_coordinates(),
        blocks,
        &valid_blocks,
    ) else {
        return None;
    };

    println!("Found reactor!");

    println!("Controller: {controller_coords}");
    println!("Back wall: {back_wall_coords}");
    println!("Right wall: {right_wall_coords}");
    println!("Left wall: {left_wall_coords}");
    println!("Up wall: {up_wall_coords}");
    println!("Down wall: {down_wall_coords}");

    Some(ReactorBounds {
        negative_coords: BlockCoordinate::new(
            controller_coords
                .x
                .min(back_wall_coords.x)
                .min(right_wall_coords.x)
                .min(left_wall_coords.x)
                .min(up_wall_coords.x)
                .min(down_wall_coords.x),
            controller_coords
                .y
                .min(back_wall_coords.y)
                .min(right_wall_coords.y)
                .min(left_wall_coords.y)
                .min(up_wall_coords.y)
                .min(down_wall_coords.y),
            controller_coords
                .z
                .min(back_wall_coords.z)
                .min(right_wall_coords.z)
                .min(left_wall_coords.z)
                .min(up_wall_coords.z)
                .min(down_wall_coords.z),
        ),
        positive_coords: BlockCoordinate::new(
            controller_coords
                .x
                .max(back_wall_coords.x)
                .max(right_wall_coords.x)
                .max(left_wall_coords.x)
                .max(up_wall_coords.x)
                .max(down_wall_coords.x),
            controller_coords
                .y
                .max(back_wall_coords.y)
                .max(right_wall_coords.y)
                .max(left_wall_coords.y)
                .max(up_wall_coords.y)
                .max(down_wall_coords.y),
            controller_coords
                .z
                .max(back_wall_coords.z)
                .max(right_wall_coords.z)
                .max(left_wall_coords.z)
                .max(up_wall_coords.z)
                .max(down_wall_coords.z),
        ),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactorValidity {
    Valid,
    TooManyControllers(BlockCoordinate),
    MissingCasing(BlockCoordinate),
}

fn check_block(
    coords: BlockCoordinate,
    valid_blocks: &[&Block],
    controller_block: &Block,
    structure: &Structure,
    blocks: &Registry<Block>,
    controller_coords: &mut Option<BlockCoordinate>,
) -> ReactorValidity {
    let block_here = structure.block_at(coords, blocks);
    if controller_block == block_here {
        if let Some(controller_coords) = controller_coords {
            if *controller_coords != coords {
                return ReactorValidity::TooManyControllers(coords);
            }
        } else {
            *controller_coords = Some(coords);
        }
    } else if !valid_blocks.contains(&block_here) {
        return ReactorValidity::MissingCasing(coords);
    }

    ReactorValidity::Valid
}

fn check_valid(bounds: ReactorBounds, structure: &Structure, blocks: &Registry<Block>) -> ReactorValidity {
    let mut controller_location = None;

    let valid_blocks = [
        blocks.from_id("cosmos:reactor_casing").expect("Missing reactor casing"),
        blocks.from_id("cosmos:reactor_window").expect("missing cosmos:reactor_window"),
    ];

    let controller_block = blocks
        .from_id("cosmos:reactor_controller")
        .expect("Missing cosmos:reactor_controller");

    for z in bounds.negative_coords.z..=bounds.positive_coords.z {
        for y in bounds.negative_coords.y..=bounds.positive_coords.y {
            let res = check_block(
                BlockCoordinate::new(bounds.negative_coords.x, y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(bounds.positive_coords.x, y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    for y in bounds.negative_coords.y..=bounds.positive_coords.y {
        for x in bounds.negative_coords.x..=bounds.positive_coords.x {
            let res = check_block(
                BlockCoordinate::new(x, y, bounds.negative_coords.z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(x, y, bounds.positive_coords.z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    for z in bounds.negative_coords.z..=bounds.positive_coords.z {
        for x in bounds.negative_coords.x..=bounds.positive_coords.x {
            let res = check_block(
                BlockCoordinate::new(x, bounds.negative_coords.y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(x, bounds.positive_coords.y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    ReactorValidity::Valid
}

fn create_reactor(
    structure: &Structure,
    blocks: &Registry<Block>,
    reactor_blocks: &Registry<ReactorPowerGenerationBlock>,
    bounds: ReactorBounds,
    controller: StructureBlock,
) -> Reactor {
    let mut power_per_second = 0.0;

    for block in structure.block_iter(bounds.negative_coords.into(), bounds.positive_coords.into(), true) {
        let block = block.block(structure, blocks);

        if let Some(reactor_block) = reactor_blocks.for_block(block) {
            power_per_second += reactor_block.power_per_second();
        }
    }

    Reactor::new(controller, power_per_second, bounds)
}

fn on_interact_reactor(
    mut structure_query: Query<(&mut Structure, &mut Reactors)>,
    blocks: Res<Registry<Block>>,
    reactor_blocks: Res<Registry<ReactorPowerGenerationBlock>>,
    mut interaction: EventReader<BlockInteractEvent>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    for ev in interaction.iter() {
        let Ok((mut structure, mut reactors)) = structure_query.get_mut(ev.structure_entity) else {
            continue;
        };

        let block = structure.block_at(ev.structure_block.coords(), &blocks);

        if block.unlocalized_name() == "cosmos:reactor_controller" {
            println!("You clicked the reactor!!!");

            if reactors.iter().any(|reactor| reactor.controller_block() == ev.structure_block) {
                continue;
            }

            if let Some(bounds) = check_is_valid_multiblock(&structure, ev.structure_block.coords(), &blocks) {
                match check_valid(bounds, &structure, &blocks) {
                    ReactorValidity::MissingCasing(coords) => structure.set_block_at(
                        coords,
                        blocks.from_id("cosmos:ship_hull_yellow").unwrap(),
                        BlockFace::Top,
                        &blocks,
                        Some(&mut event_writer),
                    ),
                    ReactorValidity::TooManyControllers(coords) => structure.set_block_at(
                        coords,
                        blocks.from_id("cosmos:ship_hull_red").unwrap(),
                        BlockFace::Top,
                        &blocks,
                        Some(&mut event_writer),
                    ),
                    ReactorValidity::Valid => {
                        let reactor = create_reactor(&structure, &blocks, &reactor_blocks, bounds, ev.structure_block);
                        reactors.add_reactor(reactor);
                    }
                };
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_interact_reactor.run_if(in_state(GameState::Playing)));
}
