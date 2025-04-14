use bevy::prelude::*;
use cosmos_core::quest::Quest;

use crate::lang::register_lang;

pub(super) fn register(app: &mut App) {
    register_lang::<Quest>(app, vec!["quests"]);
}
