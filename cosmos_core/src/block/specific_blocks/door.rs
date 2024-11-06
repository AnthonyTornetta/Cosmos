use crate::structure::chunk::BlockInfo;

pub trait DoorData {
    fn is_open(&self) -> bool;
    fn set_open(&mut self);
    fn set_closed(&mut self);
}

const DOOR_BIT: u8 = 1 << 7;

impl DoorData for BlockInfo {
    fn is_open(&self) -> bool {
        self.0 & DOOR_BIT == 0
    }

    fn set_open(&mut self) {
        self.0 &= !DOOR_BIT;
    }

    fn set_closed(&mut self) {
        self.0 |= DOOR_BIT;
    }
}
