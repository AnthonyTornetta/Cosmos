//! Handles logic blocks

use bevy::{
    app::App,
    prelude::{IntoSystemSetConfigs, SystemSet},
    state::state::OnEnter,
};
use cosmos_core::state::GameState;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Logic blocks should be registered here and can be ambiguous with this set
pub enum LogicSystemRegistrySet {
    /// Logic blocks should be registered here and can be ambiguous with this set
    RegisterLogicBlocks,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        OnEnter(GameState::PostLoading),
        LogicSystemRegistrySet::RegisterLogicBlocks.ambiguous_with(LogicSystemRegistrySet::RegisterLogicBlocks),
    );
}
