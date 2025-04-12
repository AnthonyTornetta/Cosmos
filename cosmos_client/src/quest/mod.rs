//! Client quest logic

use bevy::prelude::*;
use cosmos_core::quest::OngoingQuestId;

mod lang;
mod ui;
mod waypoint;

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// The player will have this if they currently have this quest selected.
///
/// This does NOT mean they can only do this quest - rather they have this one focused on at the
/// moment for information reasons. We should display information relevant to this quest in places
/// that make sense.
pub struct ActiveQuest(pub OngoingQuestId);

pub(super) fn register(app: &mut App) {
    ui::register(app);
    lang::register(app);
    waypoint::register(app);

    app.register_type::<ActiveQuest>();
}
