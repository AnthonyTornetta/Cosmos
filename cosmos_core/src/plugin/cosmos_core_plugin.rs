//! This should contain everything needed for a cosmos application to run

use crate::netty::sync::registry::RegistrySyncInit;
use crate::{block, chat, debug, economy, ecs, fluid, inventory, logic, netty, persistence, projectiles, shop, universe, utils};
use crate::{blockitems, structure};
use crate::{events, loader};
use crate::{item, physics};
use bevy::app::PluginGroupBuilder;
use bevy::prelude::{App, Plugin, PluginGroup, States};
use bevy::state::state::FreelyMutableState;
use bevy_easy_compute::prelude::AppComputePlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

/// This plugin group should contain everything needed for a cosmos application to run
pub struct CosmosCorePluginGroup<T>
where
    T: States + Clone + Copy + FreelyMutableState,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_game_state: T,
    registry_sync_init: RegistrySyncInit<T>,
}

/// This plugin should contain everything needed for a cosmos application to run
pub struct CosmosCorePlugin<T>
where
    T: States + Clone + Copy + FreelyMutableState,
{
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_loading_state: T,
    playing_state: T,

    registry_sync_init: RegistrySyncInit<T>,
}

impl<T: States + Clone + Copy + FreelyMutableState> CosmosCorePlugin<T> {
    /// Creates the plugin with the given states
    pub fn new(
        pre_loading_state: T,
        loading_state: T,
        post_loading_state: T,
        done_loading_state: T,
        playing_game_state: T,
        registry_sync_init: RegistrySyncInit<T>,
    ) -> Self {
        Self {
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_loading_state,
            registry_sync_init,
            playing_state: playing_game_state,
        }
    }
}

impl<T: States + Clone + Copy + FreelyMutableState> CosmosCorePluginGroup<T> {
    /// Creates the plugin group with the given states
    pub fn new(
        pre_loading_state: T,
        loading_state: T,
        post_loading_state: T,
        done_loading_state: T,
        playing_game_state: T,

        registry_sync_init: RegistrySyncInit<T>,
    ) -> Self {
        Self {
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_loading_state,
            playing_game_state,
            registry_sync_init,
        }
    }
}

impl<T: States + Clone + Copy + FreelyMutableState> Plugin for CosmosCorePlugin<T> {
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
            self.playing_state,
        );
        item::register(app, self.loading_state);
        blockitems::register(app, self.loading_state);
        physics::register(app, self.post_loading_state);
        events::register(app, self.playing_state);
        structure::register(app);
        inventory::register(app, self.playing_state);
        projectiles::register(app);
        ecs::register(app);
        persistence::register(app);
        universe::register(app);
        netty::register(app, self.registry_sync_init);
        economy::register(app);
        shop::register(app);
        logic::register(app, self.playing_state);
        fluid::register(app);
        debug::register(app);
        utils::register(app);
        chat::register(app);
    }
}

impl<T: States + Clone + Copy + FreelyMutableState> PluginGroup for CosmosCorePluginGroup<T> {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            // .add(LogPlugin::default())
            // .add(TaskPoolPlugin::default())
            // .add(TypeRegistrationPlugin::default())
            // .add(FrameCountPlugin::default())
            // .add(TimePlugin::default())
            // .add(TransformPlugin::default())
            // .add(HierarchyPlugin::default())
            // .add(DiagnosticsPlugin::default())
            // .add(InputPlugin::default())
            // .add(WindowPlugin::default())
            // .add(AccessibilityPlugin)
            // .add(AssetPlugin::default())
            // .add(ScenePlugin::default())
            // .add(RenderPlugin::default())
            // .add(ImagePlugin::default_nearest())
            .add(AppComputePlugin)
            .add(WorldInspectorPlugin::default())
            .add(CosmosCorePlugin::new(
                self.pre_loading_state,
                self.loading_state,
                self.post_loading_state,
                self.done_loading_state,
                self.playing_game_state,
                self.registry_sync_init,
            ))
    }
}
