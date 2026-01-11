//! Lets you see the fancy network sent/received graph

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use bevy_renet::renet::RenetServer;
use cosmos_core::state::GameState;
use renet_visualizer::RenetServerVisualizer;

use crate::settings::ServerSettings;

fn update_visulizer_system(
    q_windows: Query<(), With<Window>>,
    mut egui_context: EguiContexts,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    server: Res<RenetServer>,
) {
    if !q_windows.is_empty() {
        visualizer.update(&server);
        if let Ok(ctx) = egui_context.ctx_mut() {
            visualizer.show_window(ctx);
        } else {
            error!("Couldn't get egui context ;(");
        }
    }
}

pub(super) fn register(app: &mut App) {
    let settings = app.world().get_resource::<ServerSettings>().expect("Missing Settings");

    if settings.debug_window {
        app.insert_resource(RenetServerVisualizer::<200>::default())
            .allow_ambiguous_resource::<RenetServerVisualizer<200>>()
            .add_systems(EguiPrimaryContextPass, update_visulizer_system.run_if(in_state(GameState::Playing)));
    }
}
