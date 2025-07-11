use bevy::{
    color::palettes::css,
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    prelude::*,
};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    netty::client::LocalPlayer,
    quest::{OngoingQuest, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    quest::ActiveQuest,
    ui::{constants, font::DefaultFont},
};

#[derive(Component)]
struct ActiveMissionDisplay;

fn display_active_mission(
    q_display: Query<Entity, With<ActiveMissionDisplay>>,
    mut commands: Commands,
    q_changed_active_quest: Query<(&OngoingQuests, &ActiveQuest), (Or<(Changed<ActiveQuest>, Changed<OngoingQuests>)>, With<LocalPlayer>)>,
    mut removed_active_quest: RemovedComponents<ActiveQuest>,
    quests: Res<Registry<Quest>>,
    quests_lang: Res<Lang<Quest>>,
    font: Res<DefaultFont>,
    inputs: InputChecker,
) {
    if removed_active_quest.read().next().is_some() || !q_changed_active_quest.is_empty() {
        if let Ok(ent) = q_display.single() {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }

    let Ok((ongoing_quests, active_quest)) = q_changed_active_quest.single() else {
        return;
    };
    let Some(ongoing_quest) = ongoing_quests.from_id(&active_quest.0) else {
        error!("Invalid active quest id ({:?}) - not found in {ongoing_quests:?}!", active_quest.0);
        return;
    };

    let quest = quests.from_numeric_id(ongoing_quest.quest_id());

    commands
        .spawn((
            ActiveMissionDisplay,
            Name::new("Active Mission Display"),
            Node {
                width: Val::Px(500.0),
                right: Val::Px(0.0),
                top: Val::Percent(10.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("ACTIVE MISSION"),
                TextFont {
                    font: font.get(),
                    font_size: 32.0,
                    ..Default::default()
                },
            ));

            p.spawn((
                Text::new(format!(
                    "Press {} to view all quests.",
                    inputs
                        .get_control(CosmosInputs::ToggleQuestsUi)
                        .map(|x| x.to_string())
                        .unwrap_or("<unbound>".to_owned())
                )),
                TextFont {
                    font: font.get(),
                    font_size: 12.0,
                    ..Default::default()
                },
            ));

            display_quest(p, &quests_lang, &font, false, quest, &quests, ongoing_quest);
        });
}

fn display_quest<R: Relationship>(
    p: &mut RelatedSpawnerCommands<'_, R>,
    quests_lang: &Lang<Quest>,
    font: &DefaultFont,
    subquest: bool,
    quest: &Quest,
    quests: &Registry<Quest>,
    ongoing_quest: &OngoingQuest,
) {
    let text_font = TextFont {
        font: font.get(),
        font_size: if subquest { 16.0 } else { 24.0 },
        ..Default::default()
    };

    let mut ecmds = p.spawn((Text::new(quests_lang.get_name_or_unlocalized(quest)), text_font.clone()));

    let complete = ongoing_quest.completed();

    if ongoing_quest.max_progress() == 1 || complete {
        ecmds.with_children(|p| {
            p.spawn((
                TextColor(Color::from(if complete { css::GREEN } else { css::RED })),
                Text::new(format!(" {}", if complete { constants::CHECK } else { constants::CROSS })),
                text_font.clone(),
            ));
        });
    }

    if ongoing_quest.max_progress() > 1 {
        ecmds.with_children(|p| {
            p.spawn((
                TextColor(Color::from(if complete { css::GREEN } else { css::RED })),
                text_font.clone(),
            ));
        });
    }

    if let Some(subquests) = ongoing_quest.subquests() {
        p.spawn(
            (Node {
                flex_grow: 1.0,
                margin: UiRect::left(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }),
        )
        .with_children(|p| {
            for subquest in subquests.iter() {
                let quest = quests.from_numeric_id(subquest.quest_id());

                display_quest(p, quests_lang, font, true, quest, quests, subquest);
            }
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        display_active_mission
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing)),
    );
}
