//! The game's logic system: for wires, logic gates, etc.

use bevy::{app::App, prelude::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::{netty::sync::IdentifiableComponent, structure::chunk::BlockInfo};

#[derive(Component, Clone, Copy, Reflect, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
/// The logic signal this block is holding.
///
/// NOTE: Each block might interact with this data slightly differently.
///
/// Usually, a block with an output port will calculate this value the frame before outputting it and store it here.
pub struct BlockLogicData(pub i32);

impl BlockLogicData {
    /// For Boolean applications. 0 is "off" or "false", anything else is "on" or "true".
    pub fn on(&self) -> bool {
        self.0 != 0
    }
}

/// If this can be set to on or off states, this trait can be used.
///
/// For instance, [`BlockInfo`] can use the last bit to represent on/off for logic blocks.
pub trait HasOnOffInfo {
    /// Determins if this is in the `on` state
    fn on(&self) -> bool;
    /// Determins if this is in the `off` state
    fn off(&self) -> bool {
        !self.on()
    }

    /// Sets this to be in its `on` state
    fn set_on(&mut self);
    /// Sets this to be in its `off` state
    fn set_off(&mut self);
}

const LOGIC_BIT: u8 = 1 << 7;

impl HasOnOffInfo for BlockInfo {
    fn on(&self) -> bool {
        self.0 & LOGIC_BIT != 0
    }

    fn set_on(&mut self) {
        self.0 |= LOGIC_BIT;
    }

    fn set_off(&mut self) {
        self.0 &= !LOGIC_BIT;
    }
}

impl IdentifiableComponent for BlockLogicData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:block_logic_data"
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<BlockLogicData>();
}
