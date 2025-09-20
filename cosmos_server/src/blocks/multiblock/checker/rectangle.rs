use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use cosmos_core::{
    block::{
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_face::BlockFace,
        block_rotation::BlockRotation,
    },
    prelude::{BlockCoordinate, ChunkCoordinate, Structure, UnboundBlockCoordinate},
    structure::coordinates::{CoordinateType, UnboundCoordinateType},
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
    TooSmall,
}

fn connections(coord: BlockCoordinate, structure: &Structure, valid_blocks: &[u16]) -> Vec<BlockCoordinate> {
    ALL_BLOCK_DIRECTIONS
        .iter()
        .flat_map(|d| BlockCoordinate::try_from(coord + d.to_coordinates()))
        .filter(|&c| structure.is_within_blocks(c) && valid_blocks.contains(&structure.block_id_at(c)))
        .collect::<Vec<_>>()
}

pub fn check_is_valid_multiblock_bounds(
    structure: &Structure,
    starting_block: BlockCoordinate,
    valid_blocks: &[u16],
    min_size: usize,
    max_size: usize,
) -> Result<RectangleMultiblockBounds, RectangleMultiblockError> {
    let mut doing = vec![starting_block];
    let mut will_do = vec![];
    let mut already_done = HashSet::<BlockCoordinate>::default();
    already_done.insert(starting_block);

    // If it's a valid rectangular prism, each direction will be noted 4 times exactly.
    let mut found_dirs = HashMap::<BlockDirection, u32>::default();

    let mut corners = vec![];

    while !doing.is_empty() {
        for coord in doing {
            let neighbors = connections(coord, structure, valid_blocks);

            info!("{coord} - {neighbors:?}");

            if neighbors.len() > 3 {
                return Err(RectangleMultiblockError::InvalidSquare(neighbors.last().copied()));
            }

            if neighbors.len() == 3 {
                corners.push(coord);
                for neighbor in neighbors.iter().copied() {
                    let dir = BlockDirection::from_coordinates(neighbor - coord);

                    let val = found_dirs.entry(dir).or_default();
                    if *val == 4 {
                        return Err(RectangleMultiblockError::InvalidSquare(Some(neighbor)));
                    }
                    *val += 1;
                }
            }

            for neighbor in neighbors {
                if already_done.contains(&neighbor) {
                    continue;
                }
                already_done.insert(neighbor);
                will_do.push(neighbor);
            }
        }

        doing = will_do;
        will_do = vec![];
    }

    for (dir, dir_count) in found_dirs {
        if dir_count != 4 {
            error!("Missing dir: {dir:?} (only found {dir_count} times)");
            return Err(RectangleMultiblockError::InvalidSquare(None));
        }
    }

    if corners.len() > 8 {
        return Err(RectangleMultiblockError::InvalidSquare(corners.last().copied()));
    }

    if corners.len() != 8 {
        return Err(RectangleMultiblockError::InvalidSquare(None));
    }

    let mut bounds = RectangleMultiblockBounds {
        positive_coords: corners[0],
        negative_coords: corners[0],
    };

    for corner in corners.into_iter().skip(1) {
        bounds.negative_coords.x = bounds.negative_coords.x.min(corner.x);
        bounds.positive_coords.x = bounds.positive_coords.x.max(corner.x);

        bounds.negative_coords.y = bounds.negative_coords.y.min(corner.y);
        bounds.positive_coords.y = bounds.positive_coords.y.max(corner.y);

        bounds.negative_coords.z = bounds.negative_coords.z.min(corner.z);
        bounds.positive_coords.z = bounds.positive_coords.z.max(corner.z);
    }

    if bounds.positive_coords.x - bounds.negative_coords.x < (min_size as CoordinateType)
        || bounds.positive_coords.y - bounds.negative_coords.y < (min_size as CoordinateType)
        || bounds.positive_coords.z - bounds.negative_coords.z < (min_size as CoordinateType)
    {
        return Err(RectangleMultiblockError::TooSmall);
    }
    if bounds.positive_coords.x - bounds.negative_coords.x > (max_size as CoordinateType)
        || bounds.positive_coords.y - bounds.negative_coords.y > (max_size as CoordinateType)
        || bounds.positive_coords.z - bounds.negative_coords.z > (max_size as CoordinateType)
    {
        return Err(RectangleMultiblockError::TooBig);
    }

    let total_checked = already_done.len();

    let perimeter = {
        let diff = bounds.positive_coords - bounds.negative_coords;
        // This is over by 4, and idk why, thus the -4.
        (4 * (diff.x + diff.y + diff.z) - 4) as usize
    };

    if total_checked != perimeter {
        // If we haven't checked the full perimeter and nothing more, then there was a hole somewhere, or extra
        // blocks somewhere.
        error!("Perimeter error: {total_checked} vs {perimeter} ({bounds:?})");
        return Err(RectangleMultiblockError::InvalidSquare(None));
    }

    Ok(bounds)
}

pub fn check_is_valid_multiblock_bounds_bad(
    structure: &Structure,
    controller_coords: BlockCoordinate,
    valid_blocks: &[u16],
    min_size: usize,
    max_size: usize,
    start_check_dir: BlockDirection,
) -> Result<RectangleMultiblockBounds, RectangleMultiblockError> {
    debug_assert!(min_size < max_size);

    if !structure.is_within_blocks(controller_coords) {
        return Err(RectangleMultiblockError::InvalidSquare(Some(controller_coords)));
    }

    let other_axis = match start_check_dir {
        BlockDirection::PosX | BlockDirection::NegX => [
            (BlockDirection::PosY, BlockDirection::NegY),
            (BlockDirection::PosZ, BlockDirection::NegZ),
        ],
        BlockDirection::PosY | BlockDirection::NegY => [
            (BlockDirection::PosX, BlockDirection::NegX),
            (BlockDirection::PosZ, BlockDirection::NegZ),
        ],
        BlockDirection::PosZ | BlockDirection::NegZ => [
            (BlockDirection::PosX, BlockDirection::NegX),
            (BlockDirection::PosY, BlockDirection::NegY),
        ],
    };

    let ub_controller_coords = UnboundBlockCoordinate::from(controller_coords);

    let mut found_coords = None;

    {
        let search_direction = start_check_dir.to_coordinates();

        let offset = search_direction * (min_size - 1) as UnboundCoordinateType;

        // Start `min_size` back to now allow a `min_size - 1` size multiblock
        let mut check_coords = search_direction + offset;
        for _ in 0..(max_size - (min_size - 1)) {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            if !structure.is_within_blocks(check_here) {
                info!("Oob {check_here}");
                return Err(RectangleMultiblockError::InvalidSquare(Some(check_here)));
            }

            info!("Checking {check_here}");
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
        other_axis[0].0.to_coordinates(),
        other_axis[0].1.to_coordinates(),
        &valid_blocks,
        min_size,
        max_size,
    )?;

    let (down_wall_coords, up_wall_coords) = find_wall_coords(
        ub_controller_coords,
        structure,
        other_axis[1].0.to_coordinates(),
        other_axis[1].1.to_coordinates(),
        &valid_blocks,
        min_size,
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
    min_size: usize,
    max_size: usize,
) -> Result<(BlockCoordinate, BlockCoordinate), RectangleMultiblockError> {
    let mut width = 0;

    let mut found_coords = None;
    {
        let search_direction = direction_a;
        let offset = search_direction * (min_size - 1) as UnboundCoordinateType;

        let mut check_coords = search_direction + offset;
        for _ in 0..(max_size - (min_size - 1)) {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            if !structure.is_within_blocks(check_here) {
                error!("OOB {check_here}");
                return Err(RectangleMultiblockError::InvalidSquare(Some(check_here)));
            }

            width += 1;

            let block_here = structure.block_id_at(check_here);

            info!("Checking {check_here}");

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

    let Some(negative_wall_coords) = found_coords else {
        return Err(RectangleMultiblockError::TooBig);
    };

    let mut found_coords = None;
    {
        let search_direction = direction_b;
        let offset = search_direction * (min_size - 1) as UnboundCoordinateType;

        let mut check_coords = search_direction + offset;
        for _ in width..=(max_size - (min_size - 1)) {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return Err(RectangleMultiblockError::InvalidSquare(None));
            };

            if !structure.is_within_blocks(check_here) {
                error!("OOB {check_here}");
                return Err(RectangleMultiblockError::InvalidSquare(Some(check_here)));
            }

            let block_here = structure.block_id_at(check_here);
            info!("Checking {check_here}");

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

    let Some(positive_wall_coords) = found_coords else {
        return Err(RectangleMultiblockError::TooBig);
    };

    Ok((negative_wall_coords, positive_wall_coords))
}

pub(super) fn register(app: &mut App) {}
