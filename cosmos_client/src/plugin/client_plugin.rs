use crate::PluginGroup;
use bevy::app::PluginGroupBuilder;
use bevy::audio::AudioPlugin;
use bevy::gltf::GltfPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::{AnimationPlugin, GilrsPlugin};
use bevy::sprite::SpritePlugin;
use bevy::text::TextPlugin;
use bevy::ui::UiPlugin;
use bevy::winit::WinitPlugin;

#[derive(Default)]
pub struct ClientPluginGroup;

impl PluginGroup for ClientPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(WinitPlugin::default())
            .add(SpritePlugin::default())
            .add(TextPlugin::default())
            .add(UiPlugin::default())
            .add(PbrPlugin::default())
            // NOTE: Load this after renderer initialization so that it knows about the supported
            // compressed texture formats
            .add(GltfPlugin::default())
            .add(AudioPlugin::default())
            .add(GilrsPlugin::default())
            .add(AnimationPlugin::default())
    }
}
