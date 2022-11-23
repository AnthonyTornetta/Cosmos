use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::core::CorePlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::ecs::schedule::StateData;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::{App, HierarchyPlugin, ImagePlugin, Plugin, PluginGroup, TransformPlugin};
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::time::TimePlugin;
use bevy::window::WindowPlugin;
use bevy_inspector_egui::InspectableRegistry;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::block::blocks;
use crate::physics;
use crate::structure;
use crate::{events, loader};

pub struct CosmosCorePluginGroup<T>
where
    T: StateData + Clone,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

pub struct CosmosCorePlugin<T>
where
    T: StateData + Clone,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

impl<T: StateData + Clone> CosmosCorePlugin<T> {
    pub fn new(
        pre_loading_state: T,
        loading_state: T,
        post_loading_state: T,
        done_loading_state: T,
        playing_game_state: T,
    ) -> Self {
        Self {
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_loading_state,
            playing_game_state,
        }
    }
}

impl<T: StateData + Clone> CosmosCorePluginGroup<T> {
    pub fn new(
        pre_loading_state: T,
        loading_state: T,
        post_loading_state: T,
        done_loading_state: T,
        playing_game_state: T,
    ) -> Self {
        Self {
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_loading_state,
            playing_game_state,
        }
    }
}

impl<T: StateData + Clone> Plugin for CosmosCorePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(InspectableRegistry::default());

        loader::register(
            app,
            self.pre_loading_state.clone(),
            self.loading_state.clone(),
            self.post_loading_state.clone(),
            self.done_loading_state.clone(),
        );
        blocks::register(
            app,
            self.pre_loading_state.clone(),
            self.loading_state.clone(),
        );
        physics::register(app);
        structure::events::register(app);
        events::register(app, self.playing_game_state.clone());
        structure::register(
            app,
            self.post_loading_state.clone(),
            self.playing_game_state.clone(),
        );
    }
}

impl<T: StateData + Clone> PluginGroup for CosmosCorePluginGroup<T> {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        group
            .add(LogPlugin::default())
            .add(CorePlugin::default())
            .add(TimePlugin::default())
            .add(TransformPlugin::default())
            .add(ImagePlugin::default_nearest())
            .add(HierarchyPlugin::default())
            .add(DiagnosticsPlugin::default())
            .add(InputPlugin::default())
            .add(WindowPlugin::default())
            .add(AssetPlugin::default())
            .add(ScenePlugin::default())
            .add(RenderPlugin::default())
            .add(CorePipelinePlugin::default())
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(CosmosCorePlugin::new(
                self.pre_loading_state.clone(),
                self.loading_state.clone(),
                self.post_loading_state.clone(),
                self.done_loading_state.clone(),
                self.playing_game_state.clone(),
            ))
    }
}
