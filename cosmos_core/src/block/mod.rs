use bevy::{
    ecs::schedule::StateData,
    prelude::{App, Vec3},
};
use bevy_inspector_egui::Inspectable;

use crate::registry::identifiable::Identifiable;

pub mod block_builder;
pub mod blocks;

pub enum BlockProperty {
    Opaque,
    Transparent,
    Full,
    Empty,
    ShipOnly,
}

#[derive(Debug, PartialEq, Eq, Inspectable, Default, Copy, Clone)]
pub enum BlockFace {
    #[default]
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

impl BlockFace {
    pub fn index(&self) -> usize {
        match *self {
            BlockFace::Right => 0,
            BlockFace::Left => 1,
            BlockFace::Top => 2,
            BlockFace::Bottom => 3,
            BlockFace::Front => 4,
            BlockFace::Back => 5,
        }
    }

    pub fn direction(&self) -> (i32, i32, i32) {
        match *self {
            Self::Front => (0, 0, 1),
            Self::Back => (0, 0, -1),
            Self::Left => (-1, 0, 0),
            Self::Right => (1, 0, 0),
            Self::Top => (0, 1, 0),
            Self::Bottom => (0, -1, 0),
        }
    }

    pub fn direction_vec3(&self) -> Vec3 {
        match *self {
            Self::Front => Vec3::Z,
            Self::Back => Vec3::NEG_Z,
            Self::Left => Vec3::NEG_X,
            Self::Right => Vec3::X,
            Self::Top => Vec3::Y,
            Self::Bottom => Vec3::NEG_Y,
        }
    }
}

impl BlockProperty {
    fn id(&self) -> u8 {
        match *self {
            Self::Opaque => 0b1,
            Self::Transparent => 0b10,
            Self::Full => 0b100,
            Self::Empty => 0b1000,
            Self::ShipOnly => 0b10000,
        }
    }

    pub fn create_id(properties: &Vec<Self>) -> u8 {
        let mut res = 0;

        for p in properties {
            res |= p.id();
        }

        res
    }
}

pub struct Block {
    visibility: u8,
    id: u16,
    unlocalized_name: String,
    uvs: [usize; 6],
    density: f32,
}

impl Identifiable for Block {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    #[inline]
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl Block {
    pub fn new(
        properties: &Vec<BlockProperty>,
        uvs: [usize; 6],
        id: u16,
        unlocalized_name: String,
        density: f32,
    ) -> Self {
        Self {
            visibility: BlockProperty::create_id(properties),
            id,
            uvs,
            unlocalized_name,
            density,
        }
    }

    #[inline]
    pub fn is_see_through(&self) -> bool {
        self.is_transparent() || !self.is_full()
    }

    #[inline]
    pub fn is_transparent(&self) -> bool {
        self.visibility & BlockProperty::Transparent.id() != 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.visibility & BlockProperty::Full.id() != 0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.visibility & BlockProperty::Empty.id() != 0
    }

    #[inline]
    pub fn uv_index_for_side(&self, face: BlockFace) -> usize {
        self.uvs[face.index()]
    }

    #[inline]
    pub fn density(&self) -> f32 {
        self.density
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    blocks::register(app, pre_loading_state, loading_state);
}
