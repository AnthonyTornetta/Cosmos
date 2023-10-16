//! Loads assets based on the path in the given game state. Will call the done callback once everything is finished loading.

use std::marker::PhantomData;

use bevy::{
    asset::{Asset, LoadState},
    prelude::{in_state, App, AssetServer, Commands, EventWriter, Handle, IntoSystemConfigs, OnEnter, Res, ResMut, Resource, Update},
};
use cosmos_core::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};

use crate::state::game_state::GameState;

#[derive(Resource, Default)]
struct LoadingAssetHandle<T: Asset, K: Send + Sync + 'static> {
    _phantom: PhantomData<K>,
    loading: Vec<Handle<T>>,
    loading_id: usize,
}

#[derive(Resource, Default)]
struct DoneLoadingAssetHandle<T: Asset + 'static, K: Send + Sync + 'static> {
    _phantom: PhantomData<K>,
    loaded: Vec<(Handle<T>, LoadState)>,
}

/// Loads assets based on the path in the given game state. Will call the done callback once everything is finished loading.
///
/// Failed handles will also be sent to the done callback, but their LoadState will indicate if they succeeded or not
pub fn load_assets<T: Asset, K: Send + Sync + 'static>(
    app: &mut App,
    state: GameState,
    paths: Vec<&'static str>,
    done: impl Fn(Commands, Vec<(Handle<T>, LoadState)>) + Send + Sync + 'static,
) {
    let prepare_sound = move |asset_server: Res<AssetServer>,
                              mut commands: Commands,
                              mut loader: ResMut<LoadingManager>,
                              mut event_writer: EventWriter<AddLoadingEvent>| {
        let id = loader.register_loader(&mut event_writer);

        let handles = paths.iter().map(|x| asset_server.load(*x)).collect::<Vec<Handle<T>>>();

        commands.insert_resource(LoadingAssetHandle::<T, K> {
            loading: handles,
            loading_id: id,
            _phantom: Default::default(),
        });
    };

    let check_sound_done_loading = move |loading_assets: Option<ResMut<LoadingAssetHandle<T, K>>>,
                                         done_loading: Option<ResMut<DoneLoadingAssetHandle<T, K>>>,
                                         asset_server: Res<AssetServer>,
                                         mut commands: Commands,
                                         mut loader: ResMut<LoadingManager>,
                                         mut end_writer: EventWriter<DoneLoadingEvent>| {
        if let Some(mut loading_assets) = loading_assets {
            let mut done_loading = done_loading.expect("This must exist if loading exists.");

            loading_assets.loading.retain_mut(|handle| {
                let load_state = asset_server.get_load_state(handle.id());
                if load_state == LoadState::Loaded || load_state == LoadState::Failed {
                    done_loading.loaded.push((std::mem::take(handle), load_state));
                    false
                } else {
                    true
                }
            });

            if loading_assets.loading.is_empty() {
                commands.remove_resource::<LoadingAssetHandle<T, K>>();
                loader.finish_loading(loading_assets.loading_id, &mut end_writer);

                done(commands, std::mem::take(&mut done_loading.loaded));
            }
        }
    };

    app.add_systems(OnEnter(state), prepare_sound)
        .add_systems(Update, check_sound_done_loading.run_if(in_state(state)))
        .insert_resource(LoadingAssetHandle::<T, K> {
            _phantom: Default::default(),
            loading: Default::default(),
            loading_id: Default::default(),
        })
        .insert_resource(DoneLoadingAssetHandle::<T, K> {
            _phantom: Default::default(),
            loaded: Default::default(),
        });
}
