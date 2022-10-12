use bevy::{ecs::schedule::StateData, prelude::App};

pub mod block_builder;
pub mod blocks;

pub enum BlockProperty {
    Opaque,
    Transparent,
    Full,
    Empty,
    ShipOnly,
}

pub enum BlockFace {
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
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Only use this if you know what you're doing.  Should really only be used in the Blocks struct
    pub fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    #[inline]
    pub fn uv_index_for_side(&self, face: BlockFace) -> usize {
        self.uvs[face.index()]
    }

    #[inline]
    pub fn unlocalized_name(&self) -> &String {
        &self.unlocalized_name
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

pub fn register<T: StateData + Clone>(app: &mut App, pre_loading_state: T, loading_state: T) {
    blocks::register(app, pre_loading_state, loading_state);
}
