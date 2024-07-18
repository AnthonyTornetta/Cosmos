use bevy::{
    app::App,
    prelude::{IntoSystemSetConfigs, SystemSet},
    state::state::OnEnter,
};

use crate::state::GameState;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum LogicSystemRegistrySet {
    RegisterLogicBlocks,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        OnEnter(GameState::PostLoading),
        LogicSystemRegistrySet::RegisterLogicBlocks.ambiguous_with(LogicSystemRegistrySet::RegisterLogicBlocks),
    );
}
