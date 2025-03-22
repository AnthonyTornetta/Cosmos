use bevy::prelude::*;

mod parent;

pub(super) fn register(app: &mut App) {
    parent::register(app);
}
