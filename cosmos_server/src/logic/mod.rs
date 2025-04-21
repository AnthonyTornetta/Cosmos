//! Handles logic blocks

use bevy::{
    app::App,
    prelude::{IntoSystemSetConfigs, SystemSet},
    state::state::OnEnter,
};
use cosmos_core::{logic::logic_driver::LogicDriver, state::GameState};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Logic blocks should be registered here and can be ambiguous with this set
pub enum LogicSystemRegistrySet {
    /// Logic blocks should be registered here and can be ambiguous with this set
    RegisterLogicBlocks,
}

impl DefaultPersistentComponent for LogicDriver {}

pub(super) fn register(app: &mut App) {
    make_persistent::<LogicDriver>(app);

    app.configure_sets(
        OnEnter(GameState::PostLoading),
        LogicSystemRegistrySet::RegisterLogicBlocks.ambiguous_with(LogicSystemRegistrySet::RegisterLogicBlocks),
    );
}
