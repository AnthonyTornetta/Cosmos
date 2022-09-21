use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::core::CorePlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::{App, HierarchyPlugin, Plugin, PluginGroup, TransformPlugin};
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::time::TimePlugin;
use bevy::window::WindowPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::block::blocks::add_blocks_resource;
use crate::events::block_events;
use crate::physics::structure_physics;
use crate::structure;

#[derive(Default)]
pub struct CosmosCorePluginGroup;

#[derive(Default)]
pub struct CosmosCorePlugin;

impl Plugin for CosmosCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(add_blocks_resource);

        block_events::register(app);
        structure_physics::register(app);
        structure::events::register(app);
    }
}

impl PluginGroup for CosmosCorePluginGroup {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(LogPlugin::default());
        group.add(CorePlugin::default());
        group.add(TimePlugin::default());
        group.add(TransformPlugin::default());
        group.add(HierarchyPlugin::default());
        group.add(DiagnosticsPlugin::default());
        group.add(InputPlugin::default());
        group.add(WindowPlugin::default());

        group.add(AssetPlugin::default());

        group.add(ScenePlugin::default());

        group.add(RenderPlugin::default());

        group.add(CorePipelinePlugin::default());

        group.add(RapierPhysicsPlugin::<NoUserData>::default());
        group.add(CosmosCorePlugin::default());
    }
}
