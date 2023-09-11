use std::mem::swap;

use bevy::{
    prelude::{in_state, App, Commands, Deref, DerefMut, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, Update},
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{cosmos_encoder, NettyChannelServer},
    structure::lod::{Lod, LodDelta, LodNetworkMessage, SetLodMessage},
};
use futures_lite::future;

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

type LodTask = Task<Lod>;

#[derive(Debug, Resource, DerefMut, Deref, Default)]
struct ApplyingLods(Vec<(Entity, LodTask)>);

#[derive(Debug, Resource, DerefMut, Deref, Default)]
struct TodoLods(Vec<(Entity, SetLodMessage)>);

fn apply_lod_changes(mut applying_lods: ResMut<ApplyingLods>, mut commands: Commands) {
    let mut todo = Vec::with_capacity(applying_lods.capacity());

    swap(&mut applying_lods.0, &mut todo);

    for (structure_entity, mut applying_lod) in todo {
        if let Some(lod) = future::block_on(future::poll_once(&mut applying_lod)) {
            if let Some(mut ecmds) = commands.get_entity(structure_entity) {
                ecmds.insert(lod);
            }
        } else {
            applying_lods.push((structure_entity, applying_lod));
        }
    }
}

fn process_new_lods(mut todo_lods: ResMut<TodoLods>, mut applying_lods: ResMut<ApplyingLods>, lod_query: Query<&Lod>) {
    let mut todo = Vec::with_capacity(todo_lods.capacity());

    swap(&mut todo_lods.0, &mut todo);

    for todo_lod in todo {
        if applying_lods.iter().any(|(e, _)| *e == todo_lod.0) {
            todo_lods.push(todo_lod);
            continue;
        }

        let (structure_entity, lod_message) = todo_lod;

        let cur_lod = lod_query.get(structure_entity);

        let async_task_pool = AsyncComputeTaskPool::get();

        // LAG!
        let cur_lod = cur_lod.cloned();

        // This is a heavy operation, and needs to be run async
        let task = async_task_pool.spawn(async move {
            let delta_lod = cosmos_encoder::deserialize::<LodDelta>(&lod_message.serialized_lod).expect("Unable to deserialize lod delta");

            if let Ok(mut cur_lod) = cur_lod {
                delta_lod.apply_changes(&mut cur_lod);

                cur_lod
            } else {
                delta_lod.create_lod()
            }
        });

        applying_lods.0.push((structure_entity, task));
    }
}

fn listen_for_new_lods(netty_mapping: Res<NetworkMapping>, mut client: ResMut<RenetClient>, mut todo_lods: ResMut<TodoLods>) {
    while let Some(message) = client.receive_message(NettyChannelServer::DeltaLod) {
        let msg: LodNetworkMessage = cosmos_encoder::deserialize(&message).expect("Invalid LOD packet recieved from server!");

        match msg {
            LodNetworkMessage::SetLod(lod_message) => {
                if let Some(structure_entity) = netty_mapping.client_from_server(&lod_message.structure) {
                    todo_lods.push((structure_entity, lod_message));
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (listen_for_new_lods, process_new_lods, apply_lod_changes)
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<ApplyingLods>()
    .init_resource::<TodoLods>();
}
