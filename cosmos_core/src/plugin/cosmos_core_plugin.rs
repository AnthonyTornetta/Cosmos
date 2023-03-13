use bevy::a11y::AccessibilityPlugin;
use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::{
    App, FrameCountPlugin, HierarchyPlugin, ImagePlugin, Plugin, PluginGroup, States,
    TaskPoolPlugin, TransformPlugin, TypeRegistrationPlugin,
};
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::time::TimePlugin;
use bevy::window::WindowPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::{block, entities, inventory, projectiles};
use crate::{blockitems, structure};
use crate::{events, loader};
use crate::{item, physics};

pub struct CosmosCorePluginGroup<T>
where
    T: States + Clone + Copy,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

pub struct CosmosCorePlugin<T>
where
    T: States + Clone + Copy,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
}

impl<T: States + Clone + Copy> CosmosCorePlugin<T> {
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

impl<T: States + Clone + Copy> CosmosCorePluginGroup<T> {
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

impl<T: States + Clone + Copy> Plugin for CosmosCorePlugin<T> {
    fn build(&self, app: &mut App) {
        loader::register(
            app,
            self.pre_loading_state,
            self.loading_state,
            self.post_loading_state,
            self.done_loading_state,
        );

        block::register(
            app,
            self.pre_loading_state,
            self.loading_state,
            self.post_loading_state,
        );
        item::register(app);
        blockitems::register(app, self.loading_state);
        physics::register(app);
        structure::events::register(app);
        events::register(app, self.playing_game_state);
        structure::register(app, self.post_loading_state, self.playing_game_state);
        inventory::register(app);
        projectiles::register(app);
        entities::register(app);
    }
}

impl<T: States + Clone + Copy> PluginGroup for CosmosCorePluginGroup<T> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LogPlugin::default())
            .add(TaskPoolPlugin::default())
            .add(TypeRegistrationPlugin::default())
            .add(FrameCountPlugin::default())
            .add(TimePlugin::default())
            .add(TransformPlugin::default())
            .add(HierarchyPlugin::default())
            .add(DiagnosticsPlugin::default())
            .add(InputPlugin::default())
            .add(WindowPlugin::default())
            .add(AssetPlugin::default())
            .add(ScenePlugin::default())
            .add(RenderPlugin::default())
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(ImagePlugin::default_nearest())
            .add(WorldInspectorPlugin::default())
            // AccessibilityPlugin is required by bevy core ecs for some reason
            .add(AccessibilityPlugin)
            .add(CosmosCorePlugin::new(
                self.pre_loading_state,
                self.loading_state,
                self.post_loading_state,
                self.done_loading_state,
                self.playing_game_state,
            ))
    }
}
