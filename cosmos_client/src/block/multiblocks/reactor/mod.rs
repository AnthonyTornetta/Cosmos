use bevy::prelude::*;
use cosmos_core::{block::multiblock::reactor::OpenReactorEvent, netty::system_sets::NetworkingSystemsSet};

mod ui;

pub(super) fn register(app: &mut App) {
    ui::register(app);
}
