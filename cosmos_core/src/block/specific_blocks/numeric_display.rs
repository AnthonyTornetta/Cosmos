use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

/// 0-9.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NumericDisplayValue {
    #[default]
    Blank,
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Minus,
}

impl IdentifiableComponent for NumericDisplayValue {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:numeric_display_value"
    }
}

impl SyncableComponent for NumericDisplayValue {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<NumericDisplayValue>(app);
}
