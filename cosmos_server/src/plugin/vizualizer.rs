//! Lets you see the fancy network sent/received graph

use bevy::prelude::*;
// use bevy_inspector_egui::bevy_egui::EguiContexts;
// use bevy_renet::renet::RenetServer;
use renet_visualizer::RenetServerVisualizer;

// visualizer egui is bugged for now, wait till that's fixed then re-add this
fn update_visulizer_system(// q_windows: Query<(), With<Window>>,
    // mut egui_context: EguiContexts,
    // mut visualizer: ResMut<RenetServerVisualizer<200>>,
    // server: Res<RenetServer>,
) {
    // if !q_windows.is_empty() {
    // visualizer.update(&server);
    // visualizer.show_window(egui_context.ctx_mut());
    // }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(RenetServerVisualizer::<200>::default())
        .allow_ambiguous_resource::<RenetServerVisualizer<200>>()
        .add_systems(Update, update_visulizer_system);
}
