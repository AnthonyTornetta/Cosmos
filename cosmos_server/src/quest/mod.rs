//! Server quest logic

use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    netty::sync::events::server_event::NettyEventWriter,
    quest::{CompleteQuestEvent, OngoingQuestDetails, OngoingQuests},
};

use crate::persistence::{
    loading::LoadingSystemSet,
    make_persistent::{DefaultPersistentComponent, make_persistent},
};

mod quests;

#[derive(Event)]
/// Is this needed?
///
/// Send this event to add a new quest
pub struct AddQuestEvent {
    /// The unlocalized name of the quest
    pub unlocalized_name: String,
    /// The player entity that should get this quest
    pub to: Entity,
    /// The details for this quest
    /// TODO: this is stupid
    pub details: OngoingQuestDetails,
}

fn add_ongoing_quests(mut commands: Commands, q_players_no_quests: Query<Entity, (With<Player>, Without<OngoingQuests>)>) {
    for e in q_players_no_quests.iter() {
        commands.entity(e).insert(OngoingQuests::default());
    }
}

impl DefaultPersistentComponent for OngoingQuests {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum QuestsSet {
    AddOngoingQuestsComponent,
    CreateNewQuests,
    /// Quests are checked for completion, and if finished the [`CompleteQuestEvent`] is sent out
    CompleteQuests,
}

fn on_complete_quest(
    mut q_ongoing: Query<(Entity, &Player, &mut OngoingQuests), Changed<OngoingQuests>>,
    mut nevw_complete_quest_event: NettyEventWriter<CompleteQuestEvent>,
    mut evw_complete_quest_event: EventWriter<CompleteQuestEvent>,
) {
    for (entity, player, mut ongoing_quests) in q_ongoing.iter_mut() {
        let all_completed = ongoing_quests
            .iter()
            .filter(|q| q.completed())
            .map(|q| q.ongoing_id())
            .collect::<Vec<_>>();

        for completed in all_completed {
            let completed = ongoing_quests
                .remove_ongoing_quest(&completed)
                .expect("This was proven to exist above");

            let complete_quest_event = CompleteQuestEvent::new(entity, completed);

            nevw_complete_quest_event.write(complete_quest_event.clone(), player.client_id());
            evw_complete_quest_event.write(complete_quest_event);
        }
    }
}

pub(super) fn register(app: &mut App) {
    quests::register(app);

    make_persistent::<OngoingQuests>(app);

    app.configure_sets(
        FixedUpdate,
        (
            (
                QuestsSet::AddOngoingQuestsComponent.after(LoadingSystemSet::DoneLoading),
                QuestsSet::CreateNewQuests,
            )
                .chain(),
            QuestsSet::CompleteQuests,
        ),
    );

    app.add_systems(
        FixedUpdate,
        (
            add_ongoing_quests.in_set(QuestsSet::AddOngoingQuestsComponent),
            on_complete_quest.in_set(QuestsSet::CompleteQuests),
        ),
    );

    app.add_event::<AddQuestEvent>().add_event::<CompleteQuestEvent>();
}
