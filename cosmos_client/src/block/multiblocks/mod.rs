use bevy::prelude::*;
use cosmos_core::{block::multiblock::reactor::OpenReactorEvent, netty::system_sets::NetworkingSystemsSet};

fn on_receive_event(mut evr: EventReader<OpenReactorEvent>) {
    for ev in evr.read() {
        info!("Got {ev:?}");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_receive_event.in_set(NetworkingSystemsSet::Between));
}
