//! This should contain everything needed for a cosmos application to run

use crate::netty::sync::registry::RegistrySyncInit;
use crate::{
    block, chat, commands, coms, crafting, creative, debug, economy, ecs, entities, faction, fluid, inventory, logic, netty, persistence,
    projectiles, quest, shop, universe, utils,
};
use crate::{blockitems, structure};
use crate::{events, loader};
use crate::{item, physics};
use bevy::app::PluginGroupBuilder;
#[cfg(feature = "client")]
use bevy::input::common_conditions::input_toggle_active;
#[cfg(feature = "client")]
use bevy::prelude::KeyCode;
use bevy::prelude::{App, Plugin, PluginGroup, States};
use bevy::state::state::FreelyMutableState;
use bevy_app_compute::prelude::AppComputePlugin;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
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

        block::register(app, self.pre_loading_state, self.loading_state, self.post_loading_state);
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
        logic::register(app);
        fluid::register(app);
        debug::register(app);
        utils::register(app);
        chat::register(app);
        entities::register(app);
        crafting::register(app);
        coms::register(app);
        quest::register(app);
        faction::register(app);
        creative::register(app);
        commands::register(app);
    }
}

impl<T: States + Clone + Copy + FreelyMutableState> PluginGroup for CosmosCorePluginGroup<T> {
    fn build(self) -> PluginGroupBuilder {
        let mut pg = PluginGroupBuilder::start::<Self>();

        pg = pg.add(EguiPlugin {
            enable_multipass_for_primary_context: false,
        });

        #[cfg(feature = "client")]
        {
            pg = pg.add(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F2)));
        }
        #[cfg(feature = "server")]
        {
            pg = pg.add(WorldInspectorPlugin::default());
        }

        pg
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
