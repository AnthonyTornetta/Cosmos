//! Lets you see the fancy network sent/received graph

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiContexts;
use bevy_renet::renet::RenetServer;
use renet_visualizer::RenetServerVisualizer;

fn update_visulizer_system(mut egui_context: EguiContexts, mut visualizer: ResMut<RenetServerVisualizer<200>>, server: Res<RenetServer>) {
    visualizer.update(&server);
    visualizer.show_window(egui_context.ctx_mut());
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(RenetServerVisualizer::<200>::default())
        .add_system(update_visulizer_system);
}
