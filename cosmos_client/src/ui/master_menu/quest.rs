//! Quest-related UI menu

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    quest::{ActiveQuest, OngoingQuest, OngoingQuestId, OngoingQuests, Quest, SetActiveQuestMessage},
    registry::Registry,
    state::GameState,
};

use crate::{
    lang::Lang,
    ui::{
        components::{
            button::{ButtonEvent, CosmosButton},
            // scollable_container::ScrollBox,
        },
        font::DefaultFont,
    },
};

#[derive(Component)]
#[require(Node)]
/// Will render the default Quest UI on this Node entity
pub struct QuestDisplay;

fn on_add_quest_display(
    mut commands: Commands,
    default_font: Res<DefaultFont>,
    q_quests: Query<(&OngoingQuests, Option<&ActiveQuest>), With<LocalPlayer>>,
    quests: Res<Registry<Quest>>,
    lang: Res<Lang<Quest>>,
    q_ui_added: Query<Entity, Added<QuestDisplay>>,
) {
    for ent in q_ui_added.iter() {
        commands.entity(ent).with_children(|p| {
            let font = TextFont {
                font: default_font.0.clone().into(),
                font_size: FontSize::Px(24.0),
                ..Default::default()
            };

            let font_small = TextFont {
                font: default_font.0.clone().into(),
                font_size: FontSize::Px(20.0),
                ..Default::default()
            };

            let (ongoing_quests, active_quest) = q_quests
                .single()
                .map(|x| (x.0.iter().collect::<Vec<_>>(), x.1.copied()))
                .unwrap_or_default();

            if ongoing_quests.is_empty() {
                p.spawn((Text::new("No Active Missions."), font.clone()));
                p.spawn((Text::new("Hopefully a Merchant will give you one soon."), font.clone()));
                return;
            }

            p.spawn((
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                // ScrollBox { ..Default::default() },
            ))
            .with_children(|p| {
                for quest in ongoing_quests {
                    quest_node(
                        p,
                        quest,
                        &quests,
                        &lang,
                        font_small.clone(),
                        Some(quest.ongoing_id()) == active_quest.map(|x| x.0),
                    );
                }
            });
        });
    }
}

fn on_toggle_active(
    ev: On<ButtonEvent>,
    mut commands: Commands,
    mut q_active: Query<(Entity, &mut BorderColor), With<ActiveQuestUi>>,
    mut q_inactive: Query<(Entity, &QuestComp, &mut BorderColor), Without<ActiveQuestUi>>,
    mut nevw_set_active: NettyMessageWriter<SetActiveQuestMessage>,
) {
    if let Ok((ent, mut bc)) = q_active.get_mut(ev.0) {
        commands.entity(ent).remove::<ActiveQuestUi>();
        nevw_set_active.write(SetActiveQuestMessage { quest: None });
        *bc = BorderColor::all(css::LIGHT_GREY);
        return;
    }

    let Ok((ui_ent, q, mut bc)) = q_inactive.get_mut(ev.0) else {
        return;
    };

    // Replacing the selection must not briefly remove the synchronized ActiveQuest.
    for (ent, mut old_border) in &mut q_active {
        commands.entity(ent).remove::<ActiveQuestUi>();
        *old_border = BorderColor::all(css::LIGHT_GREY);
    }

    *bc = BorderColor::all(css::AQUA);
    commands.entity(ui_ent).insert(ActiveQuestUi);
    nevw_set_active.write(SetActiveQuestMessage { quest: Some(q.0) });
}

#[derive(Component)]
struct ActiveQuestUi;

#[derive(Component)]
struct QuestComp(OngoingQuestId);

fn quest_node(
    commands: &mut ChildSpawnerCommands,
    ongoing: &OngoingQuest,
    quests: &Registry<Quest>,
    lang: &Lang<Quest>,
    font: TextFont,
    active: bool,
) {
    let Some(quest) = quests.try_from_numeric_id(ongoing.quest_id()) else {
        return;
    };

    let mut ecmds = commands.spawn((
        Name::new("Quest UI Entry"),
        CosmosButton { ..Default::default() },
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(70.0),
            border: UiRect::vertical(Val::Px(2.0)),
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        QuestComp(ongoing.ongoing_id()),
        BorderColor::all(if active { css::AQUA } else { css::LIGHT_GREY }),
    ));

    ecmds.observe(on_toggle_active);

    if active {
        ecmds.insert(ActiveQuestUi);
    }
    ecmds.with_children(|p| {
        p.spawn(Node {
            flex_grow: 4.0,
            margin: UiRect::all(Val::Px(10.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn((
                Name::new("Quest Name"),
                font.clone(),
                Text::new(lang.get_name_or_unlocalized(quest).to_string()),
            ));
            p.spawn((Name::new("Quest Desc"), font.clone(), Text::new(quest.description.to_string())));
        });

        p.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            margin: UiRect::vertical(Val::Px(10.0)),
            ..Default::default()
        })
        .with_children(|p| {
            if let Some(payment) = ongoing.details.payout {
                p.spawn((
                    font.clone(),
                    Name::new("Quest Reward"),
                    TextColor(css::LIGHT_GREEN.into()),
                    Text::new(format!("+{payment} credits")),
                ));
            }

            if let Some(loc) = ongoing.details.location {
                p.spawn((
                    font.clone(),
                    TextColor(css::AQUA.into()),
                    Name::new("Quest Location"),
                    Text::new(format!("Sector: {}", loc.sector)),
                ));
            }
        });
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_quest_display.run_if(in_state(GameState::Playing)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::netty::sync::events::client_event::NettyMessageToSend;

    #[test]
    fn switching_primary_quests_sends_one_replacement_and_clicking_again_untracks() {
        let mut app = App::new();
        app.add_message::<NettyMessageToSend<SetActiveQuestMessage>>();
        let first_id = OngoingQuest::new(&Quest::new("test:first", "First quest"), Default::default(), 1).ongoing_id();
        let second_id = OngoingQuest::new(&Quest::new("test:second", "Second quest"), Default::default(), 1).ongoing_id();
        let first = app
            .world_mut()
            .spawn((QuestComp(first_id), BorderColor::all(css::LIGHT_GREY)))
            .observe(on_toggle_active)
            .id();
        let second = app
            .world_mut()
            .spawn((QuestComp(second_id), BorderColor::all(css::LIGHT_GREY)))
            .observe(on_toggle_active)
            .id();

        for (clicked, expected, other) in [
            (first, Some(first_id), second),
            (second, Some(second_id), first),
            (first, Some(first_id), second),
            (first, None, second),
        ] {
            app.world_mut().trigger(ButtonEvent(clicked));
            app.update();
            let requests = app
                .world_mut()
                .resource_mut::<Messages<NettyMessageToSend<SetActiveQuestMessage>>>()
                .drain()
                .collect::<Vec<_>>();
            assert_eq!(requests.len(), 1, "A selection should produce exactly one request");
            assert_eq!(requests[0].0.quest, expected);
            assert_eq!(app.world().get::<ActiveQuestUi>(clicked).is_some(), expected.is_some());
            assert!(app.world().get::<ActiveQuestUi>(other).is_none());
        }
    }
}
