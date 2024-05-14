use bevy::app::App;

use crate::registry::{create_registry, identifiable::Identifiable};

#[derive(Debug, Clone)]
pub struct FluidHolder {
    unlocalized_name: String,
    id: u16,
    max_capacity: f32,
}

impl Identifiable for FluidHolder {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

// fn register_fluid_holder(

// )

pub(super) fn register(app: &mut App) {
    // sync this registry
    create_registry::<FluidHolder>(app, "cosmos:fluid_holders");
}
