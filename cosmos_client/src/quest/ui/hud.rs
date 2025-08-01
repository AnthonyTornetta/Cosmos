use bevy::{
    color::palettes::css,
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    prelude::*,
};
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    netty::client::LocalPlayer,
    quest::{ActiveQuest, CompleteQuestEvent, OngoingQuest, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::{
    asset::asset_loader::load_assets,
    audio::volume::MasterVolume,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{constants, font::DefaultFont},
};

#[derive(Component)]
struct ActiveMissionDisplay;

#[derive(Component)]
struct DisplayedOngoingQuest(OngoingQuest);

fn display_active_mission(
    q_display: Query<Entity, With<ActiveMissionDisplay>>,
    mut commands: Commands,
    q_displayed_ongoing_quest: Query<&DisplayedOngoingQuest>,
    q_changed_active_quest: Query<(&OngoingQuests, &ActiveQuest), (Or<(Changed<ActiveQuest>, Changed<OngoingQuests>)>, With<LocalPlayer>)>,
    mut removed_active_quest: RemovedComponents<ActiveQuest>,
    quests: Res<Registry<Quest>>,
    quests_lang: Res<Lang<Quest>>,
    font: Res<DefaultFont>,
    inputs: InputChecker,
) {
    if (removed_active_quest.read().next().is_some() || !q_changed_active_quest.is_empty())
        && let Ok(ent) = q_display.single()
    {
        if let Ok(displayed_ongoing) = q_displayed_ongoing_quest.single()
            && let Ok((ongoing_quests, active_quest)) = q_changed_active_quest.single()
                && let Some(ongoing_quest) = ongoing_quests.from_id(&active_quest.0)
                    && ongoing_quest == &displayed_ongoing.0 {
                        // Don't rerender the quest if it's the same
                        return;
                    }
        commands.entity(ent).insert(NeedsDespawned);
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
                bottom: Val::Percent(10.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            display_quest(p, &quests_lang, &font, false, quest, &quests, ongoing_quest);

            p.spawn((
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
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
        font_size: if subquest { 20.0 } else { 32.0 },
        ..Default::default()
    };

    let text_font_desc = TextFont {
        font: font.get(),
        font_size: if subquest { 16.0 } else { 20.0 },
        ..Default::default()
    };

    let mut ecmds = p.spawn((
        Name::new("Quest Name"),
        Text::new(quests_lang.get_name_or_unlocalized(quest)),
        text_font.clone(),
    ));

    if !subquest {
        ecmds.insert(DisplayedOngoingQuest(ongoing_quest.clone()));
    }

    let complete = ongoing_quest.completed();

    if ongoing_quest.max_progress() == 1 || complete {
        ecmds.with_children(|p| {
            p.spawn((
                TextColor(Color::from(if complete { css::GREEN } else { css::RED })),
                TextSpan::new(format!(" {}", if complete { constants::CHECK } else { constants::CROSS })),
                text_font.clone(),
            ));
        });
    }

    if ongoing_quest.max_progress() > 1 {
        ecmds.with_children(|p| {
            p.spawn((
                TextColor(Color::from(if complete { css::GREEN } else { css::RED })),
                text_font.clone(),
                TextSpan::new(format!(" {}/{}", ongoing_quest.progress(), ongoing_quest.max_progress())),
            ));
        });
    }

    p.spawn((
        Name::new("Description"),
        Text::new(&quest.description),
        text_font_desc,
        Node {
            margin: UiRect::bottom(Val::Px(16.0)),
            ..Default::default()
        },
    ));

    if let Some(subquests) = ongoing_quest.subquests() {
        p.spawn((
            Name::new("Subquests"),
            Node {
                flex_grow: 1.0,
                margin: UiRect::left(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            for subquest in subquests.iter() {
                let quest = quests.from_numeric_id(subquest.quest_id());

                display_quest(p, quests_lang, font, true, quest, quests, subquest);
            }
        });
    }
}

#[derive(Component)]
struct QuestCompleteFadeout(f32);

#[derive(Component)]
struct QuestCompleteText;

const FADE_SECS: f32 = 10.0;

fn on_quest_complete(
    sound: Res<QuestCompleteSound>,
    quests: Res<Registry<Quest>>,
    lang: Res<Lang<Quest>>,
    font: Res<DefaultFont>,
    mut evr_quest_complete: EventReader<CompleteQuestEvent>,
    mut commands: Commands,
    audio: Res<Audio>,
    master_volume: Res<MasterVolume>,
) {
    for ev in evr_quest_complete.read() {
        let quest = ev.completed_quest();

        let font_style = TextFont {
            font_size: 48.0,
            font: font.get(),
            ..Default::default()
        };

        audio.play(sound.0.clone()).with_volume(master_volume.multiplier()).handle();

        commands
            .spawn((
                QuestCompleteFadeout(FADE_SECS),
                Name::new("Quest Complete"),
                Node {
                    margin: UiRect::horizontal(Val::Auto),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    font_style.clone(),
                    QuestCompleteText,
                    Text::new(format!(
                        "Mission Complete: {}",
                        lang.get_name_or_unlocalized(quests.from_numeric_id(quest.quest_id()))
                    )),
                ));

                if let Some(reward) = quest.details.payout {
                    p.spawn((
                        font_style,
                        QuestCompleteText,
                        TextColor(css::GREEN.into()),
                        Text::new(format!("+ ${reward}")),
                    ));
                }
            });
    }
}

fn fade_text(
    mut commands: Commands,
    time: Res<Time>,
    mut q_quest_complete_fade: Query<(&mut QuestCompleteFadeout, Entity)>,
    mut q_text: Query<&mut TextColor, With<QuestCompleteText>>,
) {
    for (mut fadeout, ent) in q_quest_complete_fade.iter_mut() {
        fadeout.0 -= time.delta_secs();

        if fadeout.0 <= 0.0 {
            commands.entity(ent).insert(NeedsDespawned);
            return;
        }

        for mut text_color in q_text.iter_mut() {
            text_color.0.set_alpha(fadeout.0 / FADE_SECS);
        }
    }
}

#[derive(Resource)]
struct QuestCompleteSound(Handle<AudioSource>);

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, QuestCompleteSound, 1>(
        app,
        GameState::Loading,
        ["cosmos/sounds/sfx/quest_complete.ogg"],
        |mut commands, [(sound, _)]| {
            commands.insert_resource(QuestCompleteSound(sound));
        },
    );

    app.add_systems(
        FixedUpdate,
        (on_quest_complete, display_active_mission)
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, fade_text);
}
