use bevy::prelude::*;
use cosmos_core::npc::Npc;

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

impl DefaultPersistentComponent for Npc {}

pub(super) fn register(app: &mut App) {
    make_persistent::<Npc>(app);
}
