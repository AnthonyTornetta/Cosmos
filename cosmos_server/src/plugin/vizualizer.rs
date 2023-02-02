use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiContext;
use bevy_renet::renet::RenetServer;
use renet_visualizer::RenetServerVisualizer;

fn update_visulizer_system(
    mut egui_context: ResMut<EguiContext>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    server: Res<RenetServer>,
) {
    visualizer.update(&server);
    // WAIT FOR EGUI TO UPDATE IN RENET VISUALIZER
    // visualizer.show_window(egui_context.ctx_mut());
}

pub fn register(app: &mut App) {
    app.insert_resource(RenetServerVisualizer::<200>::default())
        .add_system(update_visulizer_system);
}
