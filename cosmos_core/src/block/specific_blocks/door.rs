//! Shared door block logic

use crate::structure::chunk::BlockInfo;

/// Utility trait for interacting with door blocks and their [`BlockInfo`].
pub trait DoorData {
    /// Returns true if the door is open
    fn is_open(&self) -> bool;
    /// Sets the door state to be open
    fn set_open(&mut self);
    /// Sets the door state to be closed
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
