use bevy::prelude::*;
use cosmos_core::state::GameState;

mod logic_indicator;
mod tank;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderingModesSet {
    SetRenderingModes,
}

pub(super) fn register(app: &mut App) {
    tank::register(app);
    logic_indicator::register(app);

    app.configure_sets(
        OnEnter(GameState::PostLoading),
        RenderingModesSet::SetRenderingModes.ambiguous_with(RenderingModesSet::SetRenderingModes),
    );
}
