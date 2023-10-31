use bevy::prelude::{in_state, App, EventReader, IntoSystemConfigs, Query, Res, Update};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block, BlockFace},
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        Structure,
    },
};

use crate::state::GameState;

const MAX_SIZE: CoordinateType = 11;

fn find_wall_coords(
    ub_controller_coords: UnboundBlockCoordinate,
    structure: &Structure,
    direction_a: UnboundBlockCoordinate,
    direction_b: UnboundBlockCoordinate,
    blocks: &Registry<Block>,
) -> Option<(BlockCoordinate, BlockCoordinate)> {
    let mut width = 0;

    let mut found_coords = None;
    {
        let search_direction = direction_a;

        let mut check_coords = search_direction;
        for _ in 0..MAX_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            width += 1;

            let block_here = structure.block_at(check_here, blocks).unlocalized_name();

            if block_here != "cosmos:reactor_casing" && block_here != "cosmos:reactor_window" {
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
        for _ in width..=MAX_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            width += 1;

            let block_here = structure.block_at(check_here, blocks).unlocalized_name();

            if block_here != "cosmos:reactor_casing" && block_here != "cosmos:reactor_window" {
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

fn check_is_valid_multiblock(structure: &Structure, controller_coords: BlockCoordinate, blocks: &Registry<Block>) -> bool {
    let direction = structure.block_rotation(controller_coords);

    let ub_controller_coords = UnboundBlockCoordinate::from(controller_coords);

    let mut found_coords = None;

    {
        let search_direction = direction.local_back().direction_coordinates();

        // Start 2 back to now allow a 2x2x2 reactor - minimum size is 3x3x3
        let mut check_coords = search_direction + search_direction;
        for _ in 0..MAX_SIZE - 2 {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return false;
            };

            let block_here = structure.block_at(check_here, blocks).unlocalized_name();

            if block_here == "cosmos:reactor_casing" || block_here == "cosmos:reactor_window" {
                found_coords = Some(check_here);
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let Some(back_wall_coords) = found_coords else {
        return false;
    };

    let Some((left_wall_coords, right_wall_coords)) = find_wall_coords(
        ub_controller_coords,
        structure,
        direction.local_left().direction_coordinates(),
        direction.local_right().direction_coordinates(),
        blocks,
    ) else {
        return false;
    };

    let Some((down_wall_coords, up_wall_coords)) = find_wall_coords(
        ub_controller_coords,
        structure,
        direction.local_bottom().direction_coordinates(),
        direction.local_top().direction_coordinates(),
        blocks,
    ) else {
        return false;
    };

    println!("Found reactor!");

    println!("Controller: {controller_coords}");
    println!("Back wall: {back_wall_coords}");
    println!("Right wall: {right_wall_coords}");
    println!("Left wall: {left_wall_coords}");
    println!("Up wall: {up_wall_coords}");
    println!("Down wall: {down_wall_coords}");

    true
}

fn on_interact_reactor(structure_query: Query<&Structure>, blocks: Res<Registry<Block>>, mut interaction: EventReader<BlockInteractEvent>) {
    for ev in interaction.iter() {
        let Ok(structure) = structure_query.get(ev.structure_entity) else {
            continue;
        };

        let block = structure.block_at(ev.structure_block.coords(), &blocks);

        if block.unlocalized_name() == "cosmos:reactor_controller" {
            println!("You clicked the reactor!!!");

            check_is_valid_multiblock(structure, ev.structure_block.coords(), &blocks);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_interact_reactor.run_if(in_state(GameState::Playing)));
}
