use bevy::prelude::{App, EventReader};

use crate::events::block_events::BlockChangedEvent;

fn on_place(mut block_change_event: EventReader<BlockChangedEvent>) {}

pub(super) fn register(app: &mut App) {}
