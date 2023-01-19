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

use crate::{block, inventory, projectiles};
use crate::{blockitems, structure};
use crate::{events, loader};
use crate::{item, physics};

pub struct CosmosCorePluginGroup<T>
where
    T: StateData + Clone + Copy,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

pub struct CosmosCorePlugin<T>
where
    T: StateData + Clone + Copy,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

impl<T: StateData + Clone + Copy> CosmosCorePlugin<T> {
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

impl<T: StateData + Clone + Copy> CosmosCorePluginGroup<T> {
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

impl<T: StateData + Clone + Copy> Plugin for CosmosCorePlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(InspectableRegistry::default());

        loader::register(
            app,
            self.pre_loading_state,
            self.loading_state,
            self.post_loading_state,
            self.done_loading_state,
        );
        block::register(app, self.pre_loading_state, self.loading_state);
        item::register(app, self.pre_loading_state, self.loading_state);
        blockitems::register(app, self.pre_loading_state, self.loading_state);
        physics::register(app);
        structure::events::register(app);
        events::register(app, self.playing_game_state);
        structure::register(app, self.post_loading_state, self.playing_game_state);
        inventory::register(app);
        projectiles::register(app);
    }
}

impl<T: StateData + Clone + Copy> PluginGroup for CosmosCorePluginGroup<T> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LogPlugin::default())
            .add(CorePlugin::default())
            .add(TimePlugin::default())
            .add(TransformPlugin::default())
            .add(HierarchyPlugin::default())
            .add(DiagnosticsPlugin::default())
            .add(InputPlugin::default())
            .add(WindowPlugin::default())
            .add(AssetPlugin::default())
            .add(ScenePlugin::default())
            .add(RenderPlugin::default())
            .add(CorePipelinePlugin::default())
            // See the laser.rs file in projectiles for why &NoCollide is here.
            // I hope one day rapier updates and I don't have to use this stupidity
            // .add(RapierPhysicsPlugin::<(Option<&NoCollide>, Option<&Parent>)>::default())
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(ImagePlugin::default_nearest())
            .add(CosmosCorePlugin::new(
                self.pre_loading_state,
                self.loading_state,
                self.post_loading_state,
                self.done_loading_state,
                self.playing_game_state,
            ))
    }
}
