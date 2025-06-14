//! Custom components that don't fit the typical scheme of auto-syncing logic.
//!
//! For example, the bevy ChildOf component, that needs a bunch of custom logic to sync properly.

use bevy::prelude::*;

mod parent;

pub(super) fn register(app: &mut App) {
    parent::register(app);
}
