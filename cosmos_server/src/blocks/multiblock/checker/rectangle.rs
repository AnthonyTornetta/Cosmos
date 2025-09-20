use bevy::prelude::*;
use cosmos_core::{
    block::block_face::BlockFace,
    prelude::{BlockCoordinate, Structure, UnboundBlockCoordinate},
};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Clone, Copy, Serialize, Deserialize)]
pub struct RectangleMultiblockBounds {
    pub negative_coords: BlockCoordinate,
    pub positive_coords: BlockCoordinate,
}

struct RectangleLimit {
    pub amount: usize,
    pub block: u16,
}

pub enum RectangleMultiblockValidityError {
    BrokenLimit { block: u16 },
    MissingWall(BlockCoordinate),
}

fn check_block(
    coords: BlockCoordinate,
    valid_blocks: &[u16],
    limits: &mut [RectangleLimit],
    structure: &Structure,
) -> Option<RectangleMultiblockValidityError> {
    let block_here = structure.block_id_at(coords);
    if let Some(l) = limits.iter_mut().find(|l| l.block == block_here) {
        if l.amount == 0 {
            return Some(RectangleMultiblockValidityError::BrokenLimit { block: l.block });
        }
        l.amount -= 1;
    }

    if !valid_blocks.contains(&block_here) {
        return Some(RectangleMultiblockValidityError::MissingWall(coords));
    }

    None
}

impl RectangleMultiblockBounds {
    /// If walls are filled this returns `None` - indicating no error.
    pub fn are_walls_filled(
        &self,
        structure: &Structure,
        valid_blocks: &[u16],
        limits: &mut [RectangleLimit],
    ) -> Option<RectangleMultiblockValidityError> {
        for z in self.negative_coords.z..=self.positive_coords.z {
            for y in self.negative_coords.y..=self.positive_coords.y {
                if let Some(res) = check_block(BlockCoordinate::new(self.negative_coords.x, y, z), &valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(self.positive_coords.x, y, z), &valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        for y in self.negative_coords.y..=self.positive_coords.y {
            for x in self.negative_coords.x..=self.positive_coords.x {
                if let Some(res) = check_block(BlockCoordinate::new(x, y, self.negative_coords.z), &valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(x, y, self.positive_coords.z), &valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        for z in self.negative_coords.z..=self.positive_coords.z {
            for x in self.negative_coords.x..=self.positive_coords.x {
                if let Some(res) = check_block(BlockCoordinate::new(x, self.negative_coords.y, z), &valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(x, self.positive_coords.y, z), &valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        None
    }
}

#[derive(Error, Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum RectangleMultiblockError {
    #[display("InvalidSquare")]
    InvalidSquare(#[error(not(source))] Option<BlockCoordinate>),
    TooBig,
}

pub fn check_is_valid_multiblock_bounds(
    structure: &Structure,
    controller_coords: BlockCoordinate,
    valid_blocks: &[u16],
    min_size: usize,
    max_size: usize,
) -> Result<RectangleMultiblockBounds, RectangleMultiblockError> {
    debug_assert!(min_size < max_size);
    debug_assert!(min_size > 0);

    let rotation = structure.block_rotation(controller_coords);

    let ub_controller_coords = UnboundBlockCoordinate::from(controller_coords);

    let mut found_coords = None;

    {
        let search_direction = rotation.direction_of(BlockFace::Back).to_coordinates();

        // Start `min_size` back to now allow a `min_size - 1` size multiblock
        let mut check_coords = search_direction + search_direction;
        for _ in 0..(max_size - (min_size - 1)) {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            let block_here = structure.block_id_at(check_here);

            if valid_blocks.contains(&block_here) {
                found_coords = Some(check_here);
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let Some(back_wall_coords) = found_coords else {
        return Err(RectangleMultiblockError::TooBig);
    };

    let (left_wall_coords, right_wall_coords) = find_wall_coords(
        ub_controller_coords,
        structure,
        rotation.direction_of(BlockFace::Left).to_coordinates(),
        rotation.direction_of(BlockFace::Right).to_coordinates(),
        &valid_blocks,
        max_size,
    )?;

    let (down_wall_coords, up_wall_coords) = find_wall_coords(
        ub_controller_coords,
        structure,
        rotation.direction_of(BlockFace::Bottom).to_coordinates(),
        rotation.direction_of(BlockFace::Top).to_coordinates(),
        &valid_blocks,
        max_size,
    )?;

    Ok(RectangleMultiblockBounds {
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

fn find_wall_coords(
    ub_controller_coords: UnboundBlockCoordinate,
    structure: &Structure,
    direction_a: UnboundBlockCoordinate,
    direction_b: UnboundBlockCoordinate,
    valid_blocks: &[u16],
    max_size: usize,
) -> Result<(BlockCoordinate, BlockCoordinate), RectangleMultiblockError> {
    let mut width = 0;

    let mut found_coords = None;
    {
        let search_direction = direction_a;

        let mut check_coords = search_direction;
        for _ in 0..max_size {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            // structure.set_block_at(check_here, valid_blocks[1], Default::default(), blocks, Some(ev_writer));

            width += 1;

            let block_here = structure.block_id_at(check_here);

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
        return Err(RectangleMultiblockError::TooBig);
    };

    let mut found_coords = None;
    {
        let search_direction = direction_b;

        let mut check_coords = search_direction;
        for _ in width..=max_size {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            let block_here = structure.block_id_at(check_here);

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
        return Err(RectangleMultiblockError::TooBig);
    };

    Ok((left_wall_coords, right_wall_coords))
}

pub(super) fn register(app: &mut App) {}
