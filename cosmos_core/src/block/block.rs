use bevy_rapier3d::na::Vector2;

pub enum BlockProperty {
    Opaque,
    Transparent,
    Full,
    Empty
}

pub enum BlockFace {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom
}

impl BlockFace {
    pub fn index(&self) -> usize {
        match *self {
            BlockFace::Right => 0,
            BlockFace::Left => 1,
            BlockFace::Top => 2,
            BlockFace::Bottom => 3,
            BlockFace::Front => 4,
            BlockFace::Back => 5
        }
    }
}

impl BlockProperty {
    fn id(&self) -> u8 {
        match *self {
            BlockProperty::Opaque => 0b1,
            BlockProperty::Transparent => 0b10,
            BlockProperty::Full => 0b100,
            BlockProperty::Empty => 0b1000,
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
    uvs: [[Vector2<f32>; 2]; 6]
}

impl Block {
    pub fn new(properties: &Vec<BlockProperty>, uvs: [[Vector2<f32>; 2]; 6], id: u16, unlocalized_name: String) -> Self {
        Self {
            visibility: BlockProperty::create_id(properties),
            id,
            uvs,
            unlocalized_name
        }
    }

    pub fn is_see_through(&self) -> bool {
        self.is_transparent() || !self.is_full()
    }

    pub fn is_transparent(&self) -> bool {
        self.visibility & BlockProperty::Transparent.id() != 0
    }

    pub fn is_full(&self) -> bool {
        self.visibility & BlockProperty::Full.id() != 0
    }

    pub fn is_empty(&self) -> bool {
        self.visibility & BlockProperty::Empty.id() != 0
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn uv_for_side(&self, face: BlockFace) -> &[Vector2<f32>; 2] {
        &self.uvs[face.index()]
    }

    pub fn unlocalized_name(&self) -> &String {
        &self.unlocalized_name
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}