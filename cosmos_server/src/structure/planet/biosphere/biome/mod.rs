use std::{hash::Hash, marker::PhantomData};

use bevy::utils::HashMap;
use cosmos_core::{
    block::BlockFace,
    registry::identifiable::Identifiable,
    structure::coordinates::{BlockCoordinate, CoordinateType},
    utils::array_utils::flatten,
};

pub trait Biome: Identifiable {
    fn generate_column(&self);

    /// Gets the "y" value of a block on the planet. This "y" value is relative to the face the block is on.
    ///
    /// * `noise_generator` Used to generate noise values. Seeded for this world seed.
    /// * `(x, y, z)` Block x/y/z in the structure
    /// * `(structure_x, structure_y, structure_z)` Where the structure is in the universe - used to offset the noise values so no two structures are the same.
    /// * `(middle_air_start)` The midpoint of the extremes of heights. Aka if noise generates 0, then this should return middle_air_start.
    /// * `amplitude` Value passed in by the `GenerationParemeters`. Represents how tall the terrain will be
    /// * `delta` Value passed in by the `GenerationParemeters`. Represents how much each change in x/y/z will effect the terrain. Small values = lesser effect
    /// * `iterations` Value passed in by the `GenerationParemeters`. Represents how many times the noise function will be run
    fn get_block_height(
        &self,
        noise_generator: &noise::OpenSimplex,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> f64 {
        get_block_height(
            noise_generator,
            block_coords,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
        )
    }

    /// Returns how much the edge height should be averaged in from the other side it's approaching.
    ///
    /// Don't touch this unless you're doing something extremely crazy.
    ///
    /// - `a` x, y, or z but generalized.
    /// - `intersection` is where the two edges are projected to meet, which is used as the limit to your height.
    /// - `s_dimensions` structure width/height/length.
    fn get_mirror_coefficient(&self, a: CoordinateType, intersection: CoordinateType, s_dimensions: CoordinateType) -> f64 {
        let max = intersection;
        let min = intersection - GUIDE_MIN;
        if a > max || a < s_dimensions - max {
            1.0
        } else if a > min {
            1.0 - (max - a) as f64 / (max - min) as f64
        } else if a < s_dimensions - min {
            1.0 - ((a - (s_dimensions - max)) as f64 / (max - min) as f64)
        } else {
            0.0
        }
    }

    /// "Where the math happens" - Dan.
    ///
    /// Combining two linear gradients so that they have the same end behaviors is "a little difficult". Thus the max functions.
    ///
    /// No touchy.
    ///
    /// - `height` If you were at the center of the face of a planet - that's how tall this column would be.
    /// - `c1` The first edge coefficient (from `get_mirror_coefficient`).
    /// - `c1_height` The height on c1's edge.
    /// - `c2` The second edge coefficient (from `get_mirror_coefficient`).
    /// - `c2_height` The height on c2's edge.
    fn merge(&self, height: f64, c1: f64, c1_height: f64, c2: f64, c2_height: f64) -> CoordinateType {
        let c = if c1 + c2 == 0.0 { 0.0 } else { c1.max(c2) / (c1 + c2) };
        (height * (1.0 - c * (c1 + c2)) + c * (c1 * c1_height + c2 * c2_height)) as CoordinateType
    }

    /// Generates the "old" height, the one that's used if you're in the middle of a face.
    /// Also generates the height at any edge within GUIDE_MIN distance.
    /// Averages the "old" height with the edge heights with coefficients based on how close you are to the edge intersection.
    fn guide(
        &self,
        noise_generator: &noise::OpenSimplex,
        block_up: BlockFace,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
        s_dimensions: CoordinateType,
    ) -> CoordinateType {
        // The amplitude * iterations is an approximation to account for needing to guide the terrain farther from the edge
        // the bumpier the terrain is. Terrain may still get too bumpy.
        let top = middle_air_start - (amplitude * iterations as f64) as CoordinateType;
        let bottom = s_dimensions - top;
        let min = top - GUIDE_MIN;

        // X.
        let mut x_coefficient = 0.0;
        let mut x_height = 0.0;
        if block_coords.x > min || block_coords.x < s_dimensions - min {
            let x_coord = if block_coords.x > s_dimensions / 2 { top } else { bottom };
            let x_seed = match block_up {
                BlockFace::Front => (x_coord, block_coords.y.clamp(bottom, top), top),
                BlockFace::Back => (x_coord, block_coords.y.clamp(bottom, top), bottom),
                BlockFace::Top => (x_coord, top, block_coords.z.clamp(bottom, top)),
                BlockFace::Bottom => (x_coord, bottom, block_coords.z.clamp(bottom, top)),
                BlockFace::Right => (x_coord, block_coords.y, block_coords.z),
                BlockFace::Left => (x_coord, block_coords.y, block_coords.z),
            }
            .into();
            x_height = self::get_block_height(
                noise_generator,
                x_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            x_coefficient = Self::get_mirror_coefficient(block_coords.x, x_height as CoordinateType, s_dimensions);
        }

        // Y.
        let mut y_coefficient = 0.0;
        let mut y_height = 0.0;
        if block_coords.y > min || block_coords.y < s_dimensions - min {
            let y_coord = if block_coords.y > s_dimensions / 2 { top } else { bottom };
            let y_seed = match block_up {
                BlockFace::Front => (block_coords.x.clamp(bottom, top), y_coord, top),
                BlockFace::Back => (block_coords.x.clamp(bottom, top), y_coord, bottom),
                BlockFace::Top => (block_coords.x, y_coord, block_coords.z),
                BlockFace::Bottom => (block_coords.x, y_coord, block_coords.z),
                BlockFace::Right => (top, y_coord, block_coords.z.clamp(bottom, top)),
                BlockFace::Left => (bottom, y_coord, block_coords.z.clamp(bottom, top)),
            }
            .into();
            y_height = self::get_block_height(
                noise_generator,
                y_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            y_coefficient = Self::get_mirror_coefficient(block_coords.y, y_height as CoordinateType, s_dimensions);
        }

        // Z.
        let mut z_coefficient = 0.0;
        let mut z_height = 0.0;
        if block_coords.z > min || block_coords.z < s_dimensions - min {
            let z_coord = if block_coords.z > s_dimensions / 2 { top } else { bottom };
            let z_seed = match block_up {
                BlockFace::Front => (block_coords.x, block_coords.y, z_coord),
                BlockFace::Back => (block_coords.x, block_coords.y, z_coord),
                BlockFace::Top => (block_coords.x.clamp(bottom, top), top, z_coord),
                BlockFace::Bottom => (block_coords.x.clamp(bottom, top), bottom, z_coord),
                BlockFace::Right => (top, block_coords.y.clamp(bottom, top), z_coord),
                BlockFace::Left => (bottom, block_coords.y.clamp(bottom, top), z_coord),
            }
            .into();
            z_height = self::get_block_height(
                noise_generator,
                z_seed,
                structure_coords,
                middle_air_start,
                amplitude,
                delta,
                iterations,
            );
            z_coefficient = Self::get_mirror_coefficient(block_coords.z, z_height as CoordinateType, s_dimensions);
        }

        match block_up {
            BlockFace::Front | BlockFace::Back => Self::merge(z_height, x_coefficient, x_height, y_coefficient, y_height),
            BlockFace::Top | BlockFace::Bottom => Self::merge(y_height, x_coefficient, x_height, z_coefficient, z_height),
            BlockFace::Right | BlockFace::Left => Self::merge(x_height, y_coefficient, y_height, z_coefficient, z_height),
        }
    }

    /// Gets the top block's height
    ///
    /// * `(x, y, z)` Block x/y/z in the structure
    /// * `(structure_x, structure_y, structure_z)` Where the structure is in the universe - used to offset the noise values so no two structures are the same.
    /// * `(s_dimensions)` The width/height/length of the structure this is on.
    /// * `noise_generator` Used to generate noise values. Seeded for this world seed.
    /// * `(middle_air_start)` The midpoint of the extremes of heights. Aka if noise generates 0, then this should return middle_air_start.
    /// * `amplitude` Value passed in by the `GenerationParemeters`. Represents how tall the terrain will be
    /// * `delta` Value passed in by the `GenerationParemeters`. Represents how much each change in x/y/z will effect the terrain. Small values = lesser effect
    /// * `iterations` Value passed in by the `GenerationParemeters`. Represents how many times the noise function will be run
    fn get_top_height(
        &self,
        block_up: BlockFace,
        block_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        s_dimensions: CoordinateType,
        noise_generator: &noise::OpenSimplex,
        middle_air_start: CoordinateType,
        amplitude: f64,
        delta: f64,
        iterations: usize,
    ) -> CoordinateType {
        Self::guide(
            noise_generator,
            block_up,
            block_coords,
            structure_coords,
            middle_air_start,
            amplitude,
            delta,
            iterations,
            s_dimensions,
        )
    }
}

impl PartialEq for dyn Biome {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for dyn Biome {}

impl Hash for dyn Biome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.id())
    }
}

const LOOKUP_TABLE_PRECISION: usize = 100;
const LOOKUP_TABLE_SIZE: usize = LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION * LOOKUP_TABLE_PRECISION;

pub struct BiomeRegistry<T> {
    _phantom: PhantomData<T>,

    /// Contains a list of indicies to the biomes vec
    lookup_table: Box<[u8; LOOKUP_TABLE_SIZE]>,

    /// All the registered biomes
    biomes: Vec<Box<dyn Biome>>,
    /// Only used before `construct_lookup_table` method is called, used to store the biomes + their [`BiomeParameters`] before all the possibilities are computed.
    todo_biomes: HashMap<Box<dyn Biome>, BiomeParameters>,
}

pub struct BiomeParameters {
    /// This must be within 0.0 to 100.0
    pub ideal_temperature: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_elevation: f32,
    /// This must be within 0.0 to 100.0
    pub ideal_humidity: f32,
}

impl<T> Default for BiomeRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BiomeRegistry<T> {
    pub fn new() -> Self {
        Self {
            _phantom: Default::default(),
            lookup_table: Box::new([0; LOOKUP_TABLE_SIZE]),
            biomes: vec![],
            todo_biomes: Default::default(),
        }
    }

    fn construct_lookup_table() {}

    pub fn register(&mut self, biome: Box<dyn Biome>, params: BiomeParameters) {
        self.todo_biomes.insert(biome, params);
    }

    pub fn ideal_biome_for(&self, params: BiomeParameters) -> &dyn Biome {
        let lookup_idx = flatten(
            params.ideal_elevation as usize,
            params.ideal_humidity as usize,
            params.ideal_temperature as usize,
            LOOKUP_TABLE_PRECISION,
            LOOKUP_TABLE_PRECISION,
        );

        self.biomes[self.lookup_table[lookup_idx] as usize].as_ref()
    }
}
