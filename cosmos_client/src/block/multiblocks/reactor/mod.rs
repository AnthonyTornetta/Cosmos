use bevy::prelude::*;
use cosmos_core::{block::multiblock::reactor::OpenReactorEvent, netty::system_sets::NetworkingSystemsSet};

mod ui;

fn on_receive_event(mut evr: EventReader<OpenReactorEvent>) {
    for ev in evr.read() {
        info!("Got {ev:?}");
    }
}

pub(super) fn register(app: &mut App) {
    ui::register(app);
    app.add_systems(Update, on_receive_event.in_set(NetworkingSystemsSet::Between));
}
