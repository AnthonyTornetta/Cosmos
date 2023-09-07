use bevy::prelude::{in_state, App, Commands, IntoSystemConfigs, Query, Res, ResMut, Update};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{cosmos_encoder, NettyChannelServer},
    structure::lod::{Lod, LodDelta, LodNetworkMessage},
    utils::timer::UtilsTimer,
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

/// Note: this method will crash if the server sends 2 lod updates immediately, which could happen.
///
/// This should be updated to account for that. Perhaps send all deltas at once in a vec?
///
/// It will crash because it will insert the lod, but the query won't then return it for the next delta, which will rely on it
fn listen_for_new_lods(
    netty_mapping: Res<NetworkMapping>,
    mut lod_query: Query<&mut Lod>,
    mut client: ResMut<RenetClient>,
    mut commands: Commands,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::DeltaLod) {
        let msg: LodNetworkMessage = cosmos_encoder::deserialize(&message).expect("Invalid LOD packet recieved from server!");

        match msg {
            LodNetworkMessage::SetLod(lod) => {
                if let Some(structure_entity) = netty_mapping.client_from_server(&lod.structure) {
                    println!("Got LOD for {structure_entity:?}");
                    if let Some(mut ecmds) = commands.get_entity(structure_entity) {
                        let cur_lod = lod_query.get_mut(structure_entity);

                        let timer = UtilsTimer::start();

                        let delta_lod = cosmos_encoder::deserialize::<LodDelta>(&lod.serialized_lod).expect("Unable to deserialize lod");

                        if let Ok(mut cur_lod) = cur_lod {
                            delta_lod.apply_changes(&mut cur_lod);
                        } else {
                            let created = delta_lod.clone().create_lod();
                            if matches!(created, Lod::None) {
                                println!("From:");
                                // remove above clone when you remove this
                                println!("{delta_lod:?}");
                            }
                            ecmds.insert(created);
                        }

                        timer.log_duration("Apply LOD changes:");
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, listen_for_new_lods.run_if(in_state(GameState::Playing)));
}
