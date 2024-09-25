use bevy::{
    app::Update,
    prelude::{in_state, App, EventReader, IntoSystemConfigs},
};
use cosmos_core::{netty::system_sets::NetworkingSystemsSet, state::GameState, universe::map::system::RequestSystemMap};

fn send_map(mut evr_request_map: EventReader<RequestSystemMap>) {
    for ev in evr_request_map.read() {
        println!("{ev:?}");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        send_map.in_set(NetworkingSystemsSet::Between).run_if(in_state(GameState::Playing)),
    );
}
