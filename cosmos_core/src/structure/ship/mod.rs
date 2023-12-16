//! A ship is a structure that has velocity & is created by the player.
//!
//! Ships can also be piloted by the player.

use bevy::app::Update;
use bevy::ecs::event::EventReader;
use bevy::ecs::schedule::common_conditions::in_state;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Res;
use bevy::prelude::App;
use bevy::prelude::Component;
use bevy::prelude::States;
use bevy::reflect::Reflect;

use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;

use super::shared::MeltingDown;

pub mod pilot;
pub mod ship_builder;
pub mod ship_movement;

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// A structure that has this component is a ship
pub struct Ship;

fn monitor_block_events(mut commands: Commands, blocks: Res<Registry<Block>>, mut event_reader: EventReader<BlockChangedEvent>) {
    for ev in event_reader.read() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:ship_core" {
            commands.entity(ev.structure_entity).insert(MeltingDown::default());
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot::register(app);
    ship_movement::register(app);
    ship_builder::register(app);

    app.add_systems(Update, monitor_block_events.run_if(in_state(playing_state)))
        .register_type::<Ship>();
}
