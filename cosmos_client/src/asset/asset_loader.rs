//! Loads assets based on the path in the given game state. Will call the done callback once everything is finished loading.

use std::marker::PhantomData;

use bevy::{
    asset::{Asset, LoadState},
    prelude::{App, AssetServer, Commands, EventWriter, Handle, IntoSystemConfigs, OnEnter, Res, ResMut, Resource, Update, in_state},
};
use cosmos_core::{
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    state::GameState,
};

#[derive(Resource)]
struct LoadingAssetHandle<T: Asset, K: Send + Sync + 'static, const N: usize> {
    _phantom: PhantomData<K>,
    loading_handles: [Option<Handle<T>>; N],
    loading_id: usize,
}

#[derive(Resource)]
struct DoneLoadingAssetHandle<T: Asset + 'static, K: Send + Sync + 'static, const N: usize> {
    _phantom: PhantomData<K>,
    loaded_handles: [Option<(Handle<T>, LoadState)>; N],
}

/// Loads assets based on the path in the given game state. Will call the done callback once everything is finished loading.
///
/// Failed handles will also be sent to the done callback, but their LoadState will indicate if they succeeded or not
/// The order of the loaded asset arguments will match the order of the passed in requested assets.
///
/// Usage: [`load_assets<AssetType, AnyMarkerType>`]
///
/// The marker type is to differentiate this loading call from other loading calls, even if it's loading the same asset type.
/// You can just make a throw-away zero-sized struct for this, just make sure it's not being used by any other load_assets call in the same state.
pub fn load_assets<T: Asset, K: Send + Sync + 'static, const N: usize>(
    app: &mut App,
    state: GameState,
    paths: [&'static str; N],
    done: impl Fn(Commands, [(Handle<T>, LoadState); N]) + Send + Sync + 'static,
) {
    let prepare_assets = move |asset_server: Res<AssetServer>,
                               mut commands: Commands,
                               mut loader: ResMut<LoadingManager>,
                               mut event_writer: EventWriter<AddLoadingEvent>| {
        let id = loader.register_loader(&mut event_writer);

        let handles = paths.map(|x| Some(asset_server.load(x)));

        commands.insert_resource(LoadingAssetHandle::<T, K, N> {
            loading_handles: handles,
            loading_id: id,
            _phantom: Default::default(),
        });

        commands.insert_resource(DoneLoadingAssetHandle::<T, K, N> {
            _phantom: Default::default(),
            loaded_handles: [const { None }; N],
        });
    };

    let check_assets_done_loading = move |loading_assets: Option<ResMut<LoadingAssetHandle<T, K, N>>>,
                                          done_loading: Option<ResMut<DoneLoadingAssetHandle<T, K, N>>>,
                                          asset_server: Res<AssetServer>,
                                          mut commands: Commands,
                                          mut loader: ResMut<LoadingManager>,
                                          mut end_writer: EventWriter<DoneLoadingEvent>| {
        if let Some(mut loading_assets) = loading_assets {
            let mut done_loading = done_loading.expect("This must exist if loading exists.");

            loading_assets
                .loading_handles
                .iter_mut()
                .enumerate()
                .filter(|(_, x)| x.is_some())
                .for_each(|(idx, handle)| {
                    let h = handle.as_mut().expect("Verified in filter.");

                    let load_state = asset_server.get_load_state(h.id()).expect("Id has to exist here");

                    if matches!(load_state, LoadState::Failed(_) | LoadState::Loaded) {
                        done_loading.loaded_handles[idx] = Some((std::mem::take(handle).expect("Verified in filter"), load_state));
                    }
                });

            if loading_assets.loading_handles.iter().all(|x| x.is_none()) {
                commands.remove_resource::<LoadingAssetHandle<T, K, N>>();
                loader.finish_loading(loading_assets.loading_id, &mut end_writer);

                let loaded = done_loading
                    .loaded_handles
                    .iter_mut()
                    .map(|x| std::mem::take(x).expect("This must be Some at this point - all assets were loaded"))
                    .collect::<Vec<_>>();

                let dest: [(Handle<T>, LoadState); N] = loaded.try_into().unwrap();

                done(commands, dest);
            }
        }
    };

    app.add_systems(OnEnter(state), prepare_assets)
        .add_systems(Update, check_assets_done_loading.run_if(in_state(state)));
}
