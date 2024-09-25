use bevy::{
    app::Update,
    prelude::{in_state, App, EventReader, IntoSystemConfigs},
};
use cosmos_core::{
    netty::{
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
    universe::map::system::{RequestSystemMap, SystemMap, SystemMapResponseEvent},
};

fn send_map(
    mut evr_request_map: EventReader<NettyEventReceived<RequestSystemMap>>,
    mut nevw_system_map: NettyEventWriter<SystemMapResponseEvent>,
) {
    for ev in evr_request_map.read() {
        println!("Got: {ev:?} -- sending response!");

        nevw_system_map.send(
            SystemMapResponseEvent {
                map: SystemMap::default(),
                system: ev.system,
            },
            ev.client_id,
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        send_map.in_set(NetworkingSystemsSet::Between).run_if(in_state(GameState::Playing)),
    );
}
