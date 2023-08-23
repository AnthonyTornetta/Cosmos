use bevy::prelude::{in_state, App, Commands, IntoSystemConfigs, Res, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{cosmos_encoder, NettyChannelServer},
    structure::lod::{Lod, LodNetworkMessage},
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

fn listen_for_new_lods(netty_mapping: Res<NetworkMapping>, mut client: ResMut<RenetClient>, mut commands: Commands) {
    while let Some(message) = client.receive_message(NettyChannelServer::Lod) {
        let msg: LodNetworkMessage = cosmos_encoder::deserialize(&message).expect("Invalid LOD packet recieved from server!");

        match msg {
            LodNetworkMessage::SetLod(lod) => {
                if let Some(structure_entity) = netty_mapping.client_from_server(&lod.structure) {
                    if let Some(mut ecmds) = commands.get_entity(structure_entity) {
                        let lod = cosmos_encoder::deserialize::<Lod>(&lod.serialized_lod).expect("Unable to deserialize lod");

                        ecmds.insert(lod);
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, listen_for_new_lods.run_if(in_state(GameState::Playing)));
}
