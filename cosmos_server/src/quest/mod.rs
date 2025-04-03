use bevy::prelude::*;
use cosmos_core::quest::OngoingQuestDetails;

mod quests;

#[derive(Event)]
pub struct AddQuestEvent {
    pub unlocalized_name: String,
    pub to: Entity,
    pub details: OngoingQuestDetails,
}

pub(super) fn register(app: &mut App) {
    quests::register(app);

    app.add_event::<AddQuestEvent>();
}
