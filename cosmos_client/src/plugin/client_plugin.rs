//! Every plugin needed for the client to run.

use crate::PluginGroup;
use bevy::app::PluginGroupBuilder;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::gltf::GltfPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::AnimationPlugin;
use bevy::text::TextPlugin;
use bevy::ui::UiPlugin;
use bevy::winit::{WakeUp, WinitPlugin};

#[derive(Default)]
/// Every plugin needed for the client to run.
pub struct ClientPluginGroup;

impl PluginGroup for ClientPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(WinitPlugin::<WakeUp>::default())
            .add(TextPlugin)
            .add(UiPlugin)
            .add(PbrPlugin::default())
            .add(CorePipelinePlugin)
            // NOTE: Load this after renderer initialization so that it knows about the supported
            // compressed texture formats
            .add(GltfPlugin::default())
            .add(AnimationPlugin)
    }
}
