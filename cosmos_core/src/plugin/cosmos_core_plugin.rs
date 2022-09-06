use bevy::app::PluginGroupBuilder;
use bevy::asset::AssetPlugin;
use bevy::audio::AudioPlugin;
use bevy::core::CorePlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::gltf::GltfPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::{AnimationPlugin, GilrsPlugin, HierarchyPlugin, PluginGroup, TransformPlugin};
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::sprite::SpritePlugin;
use bevy::text::TextPlugin;
use bevy::time::TimePlugin;
use bevy::ui::UiPlugin;
use bevy::window::WindowPlugin;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

#[derive(Default)]
pub struct CosmosCorePluginGroup;

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
    }
}