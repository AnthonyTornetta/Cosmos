use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use cosmos_core::{
    block::block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
    prelude::{BlockCoordinate, Structure},
    structure::coordinates::CoordinateType,
};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Clone, Copy, Serialize, Deserialize)]
pub struct RectangleMultiblockBounds {
    pub negative_coords: BlockCoordinate,
    pub positive_coords: BlockCoordinate,
}

pub struct RectangleLimit {
    pub amount: usize,
    pub block: u16,
}

pub enum RectangleMultiblockValidityError {
    BrokenLimit { block: u16, coordinate: BlockCoordinate },
    InvalidBlock(BlockCoordinate),
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
            return Some(RectangleMultiblockValidityError::BrokenLimit {
                block: l.block,
                coordinate: coords,
            });
        } else {
            l.amount -= 1;
        }
    }

    if !valid_blocks.contains(&block_here) {
        return Some(RectangleMultiblockValidityError::InvalidBlock(coords));
    }

    None
}

impl RectangleMultiblockBounds {
    /// If walls are filled this returns `None` - indicating no error.
    pub fn check_walls_filled(
        &self,
        structure: &Structure,
        valid_blocks: &[u16],
        limits: &mut [RectangleLimit],
    ) -> Option<RectangleMultiblockValidityError> {
        for z in self.negative_coords.z..=self.positive_coords.z {
            for y in self.negative_coords.y..=self.positive_coords.y {
                if let Some(res) = check_block(BlockCoordinate::new(self.negative_coords.x, y, z), valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(self.positive_coords.x, y, z), valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        for y in self.negative_coords.y..=self.positive_coords.y {
            for x in self.negative_coords.x + 1..=self.positive_coords.x - 1 {
                if let Some(res) = check_block(BlockCoordinate::new(x, y, self.negative_coords.z), valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(x, y, self.positive_coords.z), valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        for z in self.negative_coords.z + 1..=self.positive_coords.z - 1 {
            for x in self.negative_coords.x + 1..=self.positive_coords.x - 1 {
                if let Some(res) = check_block(BlockCoordinate::new(x, self.negative_coords.y, z), valid_blocks, limits, structure) {
                    return Some(res);
                }

                if let Some(res) = check_block(BlockCoordinate::new(x, self.positive_coords.y, z), valid_blocks, limits, structure) {
                    return Some(res);
                }
            }
        }

        None
    }

    /// If the inside is filled, with only the valid blocks and breaks no limits, this returns `None` - indicating no error.
    pub fn check_inside_filled(
        &self,
        structure: &Structure,
        valid_blocks: &[u16],
        limits: &mut [RectangleLimit],
    ) -> Option<RectangleMultiblockValidityError> {
        for z in self.negative_coords.z + 1..=self.positive_coords.z - 1 {
            for y in self.negative_coords.y + 1..=self.positive_coords.y - 1 {
                for x in self.negative_coords.x + 1..=self.positive_coords.x - 1 {
                    if let Some(res) = check_block(BlockCoordinate::new(x, y, z), valid_blocks, limits, structure) {
                        return Some(res);
                    }
                }
            }
        }

        None
    }

    pub fn perimeter(&self) -> CoordinateType {
        let diff = self.positive_coords - self.negative_coords;
        if diff.x == 0 && diff.y == 0 && diff.z == 0 {
            return 0;
        }

        // This is over by 4, and idk why, thus the -4.
        (4 * (diff.x + diff.y + diff.z) - 4) as CoordinateType
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

    let perimeter = bounds.perimeter();

    if total_checked != perimeter as usize {
        // If we haven't checked the full perimeter and nothing more, then there was a hole somewhere, or extra
        // blocks somewhere.
        error!("Perimeter error: {total_checked} vs {perimeter} ({bounds:?})");
        return Err(RectangleMultiblockError::InvalidSquare(None));
    }

    Ok(bounds)
}

pub(super) fn register(app: &mut App) {}
