use bevy::app::{App, PluginGroup, PluginGroupBuilder};
use bevy::core::CorePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::time::TimePlugin;
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
        group.add(RapierPhysicsPlugin::<NoUserData>::default());
    }
}