//! Server quest logic

use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
    },
    quest::{ActiveQuest, CompleteQuestEvent, OngoingQuestDetails, OngoingQuests, SetActiveQuestEvent},
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
/// The system sets Quests systems should run in
pub enum QuestsSet {
    /// Adds the [`OngoingQuests`] component if the player doesn't have one.
    AddOngoingQuestsComponent,
    /// Creates new [`cosmos_core::quest::OngoingQuest`]s and adds them to the [`OngoingQuests`] component.
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

impl DefaultPersistentComponent for ActiveQuest {}

fn clear_invalid_active_quest(q_active: Query<(Entity, &OngoingQuests, &ActiveQuest)>, mut commands: Commands) {
    for (ent, ongoing, active) in q_active.iter() {
        if !ongoing.contains_ongoing(&active.0) {
            commands.entity(ent).remove::<ActiveQuest>();
        }
    }
}

fn on_set_ongoing(
    mut nevr_ongoing_quest: EventReader<NettyEventReceived<SetActiveQuestEvent>>,
    lobby: Res<ServerLobby>,
    mut commands: Commands,
    q_ongoing: Query<&OngoingQuests>,
) {
    for ev in nevr_ongoing_quest.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        match ev.quest {
            Some(q) => {
                let Ok(ongoing) = q_ongoing.get(player) else {
                    error!("Player {player:?} did not have ongoing quests!");
                    continue;
                };

                if !ongoing.contains_ongoing(&q) {
                    error!("Player set quest they didn't have as their active one!");
                    continue;
                }

                commands.entity(player).insert(ActiveQuest(q));
            }
            None => {
                commands.entity(player).remove::<ActiveQuest>();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    quests::register(app);

    make_persistent::<OngoingQuests>(app);
    make_persistent::<ActiveQuest>(app);

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
            (clear_invalid_active_quest, on_set_ongoing)
                .chain()
                .before(QuestsSet::CompleteQuests)
                .after(QuestsSet::CreateNewQuests),
        ),
    );

    app.add_event::<AddQuestEvent>().add_event::<CompleteQuestEvent>();
}
