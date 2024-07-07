//! A station is a structure that has no velocity & is created by the player.
//!
//! They serve many pusposes, such as being a home base or a shopping center.

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Commands, Res},
    },
    reflect::Reflect,
    state::{condition::in_state, state::States},
};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
};

use super::shared::MeltingDown;

pub mod station_builder;

fn monitor_block_events(mut commands: Commands, blocks: Res<Registry<Block>>, mut event_reader: EventReader<BlockChangedEvent>) {
    for ev in event_reader.read() {
        let block = blocks.from_numeric_id(ev.old_block);

        if block.unlocalized_name() == "cosmos:station_core" {
            commands.entity(ev.structure_entity).insert(MeltingDown::default());
        }
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// A structure that has this component is a space station
pub struct Station;

pub(super) fn register<T: States + Copy>(app: &mut App, playing_state: T) {
    app.add_systems(Update, monitor_block_events.run_if(in_state(playing_state)))
        .register_type::<Station>();

    station_builder::register(app);
}
