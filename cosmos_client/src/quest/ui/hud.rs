use bevy::{
    color::palettes::css,
    ecs::relationship::{RelatedSpawnerCommands, Relationship},
    prelude::*,
};
use bevy_kira_audio::{Audio, AudioControl, AudioSource};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    netty::client::LocalPlayer,
    quest::{ActiveQuest, CompleteQuestMessage, OngoingQuest, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::{
    asset::asset_loader::load_assets,
    audio::volume::MasterVolume,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{components::scollable_container::ScrollbarStyle, constants, font::DefaultFont},
};

#[derive(Component)]
struct ActiveMissionDisplay;

#[derive(Component)]
struct DisplayedOngoingQuest(OngoingQuest);

#[derive(Component)]
struct MissionContent;

const TRACKER_BACKGROUND: Color = Color::srgba(0.067, 0.106, 0.145, 0.94);
const TRACKER_TEXT: Color = Color::srgb(0.890, 0.929, 0.949);
const TRACKER_MUTED: Color = Color::srgb(0.635, 0.718, 0.776);
const TRACKER_ACCENT: Color = Color::srgb(0.478, 0.827, 0.863);

fn display_active_mission(
    q_display: Query<(Entity, &DisplayedOngoingQuest), With<ActiveMissionDisplay>>,
    q_scroll: Query<&ScrollPosition, With<MissionContent>>,
    mut commands: Commands,
    q_active_quest: Query<(&OngoingQuests, &ActiveQuest), With<LocalPlayer>>,
    quests: Res<Registry<Quest>>,
    quests_lang: Res<Lang<Quest>>,
    font: Res<DefaultFont>,
    inputs: InputChecker,
) {
    // Reconcile against current local state; removal events can refer to another
    // player or an earlier selection. Equal snapshots below avoid rebuilding.
    let ongoing_quest = q_active_quest
        .single()
        .ok()
        .and_then(|(ongoing, active)| ongoing.from_id(&active.0));

    let mut scroll_position = ScrollPosition::default();
    for (ent, displayed) in &q_display {
        if ongoing_quest == Some(&displayed.0) {
            return;
        }
        if ongoing_quest.is_some_and(|ongoing| ongoing.ongoing_id() == displayed.0.ongoing_id()) {
            scroll_position = q_scroll.single().cloned().unwrap_or_default();
        }
        commands.entity(ent).despawn();
    }

    let Some(ongoing_quest) = ongoing_quest else {
        return;
    };

    let quest = quests.from_numeric_id(ongoing_quest.quest_id());

    commands
        .spawn((
            ActiveMissionDisplay,
            DisplayedOngoingQuest(ongoing_quest.clone()),
            DespawnOnExit(GameState::Playing),
            Name::new("Active Mission Display"),
            Node {
                width: Val::Px(350.0),
                max_width: Val::Vw(96.0),
                max_height: Val::Vh(45.0),
                right: Val::Px(12.0),
                bottom: Val::Percent(10.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                row_gap: Val::Px(8.0),
                border: UiRect::left(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..Default::default()
            },
            BackgroundColor(TRACKER_BACKGROUND),
            BorderColor::all(TRACKER_ACCENT),
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Quest tracker content"),
                MissionContent,
                Node {
                    flex_direction: FlexDirection::Column,
                    min_height: Val::Px(0.0),
                    overflow: Overflow::scroll_y(),
                    scrollbar_width: 3.0,
                    ..Default::default()
                },
                scroll_position,
                ScrollbarStyle {
                    thumb: TRACKER_MUTED,
                    gutter: TRACKER_BACKGROUND,
                    corner: Color::NONE,
                },
                Pickable::default(),
            ))
            .with_children(|p| display_quest(p, &quests_lang, &font, false, quest, &quests, ongoing_quest));

            p.spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(4.0),
                flex_shrink: 0.0,
                ..Default::default()
            })
            .with_children(|p| {
                if let Some(payout) = ongoing_quest.details.payout {
                    tracker_label(p, &font, format!("+{payout} credits"), 11.0, TRACKER_MUTED);
                }
                tracker_label(
                    p,
                    &font,
                    format!(
                        "[{}] All quests",
                        inputs
                            .get_control(CosmosInputs::ToggleQuestsUi)
                            .map(|x| x.to_string())
                            .unwrap_or("unbound".to_owned())
                    ),
                    11.0,
                    TRACKER_MUTED,
                );
            });
        });
}

fn tracker_label<R: Relationship>(
    p: &mut RelatedSpawnerCommands<'_, R>,
    font: &DefaultFont,
    text: impl Into<String>,
    size: f32,
    color: Color,
) {
    p.spawn((
        Text::new(text),
        TextFont {
            font: font.get(),
            font_size: size,
            ..Default::default()
        },
        TextColor(color),
        Node {
            min_width: Val::Px(0.0),
            flex_shrink: 0.0,
            ..Default::default()
        },
    ));
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
    let complete = ongoing_quest.completed();
    p.spawn((
        Name::new("Quest objective"),
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            ..Default::default()
        },
    ))
    .with_children(|p| {
        p.spawn(Node {
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Start,
            ..Default::default()
        })
        .with_children(|p| {
            if subquest || complete {
                tracker_label(
                    p,
                    font,
                    if complete { constants::CHECK.to_string() } else { "•".into() },
                    14.0,
                    TRACKER_ACCENT,
                );
            }
            p.spawn((
                Name::new("Quest Name"),
                Text::new(quests_lang.get_name_or_unlocalized(quest)),
                TextFont {
                    font: font.get(),
                    font_size: if subquest { 14.0 } else { 18.0 },
                    ..Default::default()
                },
                TextColor(if complete { TRACKER_MUTED } else { TRACKER_TEXT }),
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_width: Val::Px(0.0),
                    ..Default::default()
                },
            ));
            if ongoing_quest.max_progress() > 1 {
                tracker_label(
                    p,
                    font,
                    format!("{}/{}", ongoing_quest.progress(), ongoing_quest.max_progress()),
                    12.0,
                    if complete { TRACKER_MUTED } else { TRACKER_ACCENT },
                );
            }
        });

        if !quest.description.is_empty() {
            tracker_label(p, font, &quest.description, 12.0, TRACKER_MUTED);
        }

        if let Some(subquests) = ongoing_quest.subquests() {
            p.spawn((
                Name::new("Subquests"),
                Node {
                    margin: UiRect {
                        top: Val::Px(5.0),
                        left: Val::Px(if subquest { 8.0 } else { 0.0 }),
                        ..Default::default()
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    flex_shrink: 0.0,
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
    });
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
    mut evr_quest_complete: MessageReader<CompleteQuestMessage>,
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

        audio.play(sound.0.clone()).with_volume(master_volume.get()).handle();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::inputs::CosmosInputHandler;
    use cosmos_core::quest::QuestBuilder;

    fn setup() -> (App, Entity) {
        let mut quests = Registry::<Quest>::new("test:quests");
        for (name, description) in [
            ("Mission", "A mission with several simultaneous objectives."),
            ("Move", "Move your ship in each direction."),
            ("Supplies", "Prepare supplies before leaving."),
            ("Craft", "Fabricate the required parts."),
            ("Board", "Enter your ship."),
        ] {
            quests.register(Quest::new(name, description));
        }
        let main = QuestBuilder::new(quests.from_id("Mission").unwrap())
            .with_max_progress(3)
            .with_starting_progress(1)
            .with_subquests([
                QuestBuilder::new(quests.from_id("Move").unwrap())
                    .with_max_progress(100)
                    .with_starting_progress(37)
                    .build(),
                QuestBuilder::new(quests.from_id("Supplies").unwrap())
                    .with_subquests([QuestBuilder::new(quests.from_id("Craft").unwrap())
                        .with_max_progress(20)
                        .with_starting_progress(20)
                        .build()])
                    .build(),
                QuestBuilder::new(quests.from_id("Board").unwrap())
                    .with_starting_progress(1)
                    .build(),
            ])
            .build();
        let mut ongoing = OngoingQuests::default();
        let active = ongoing.start_quest(main);
        let mut input = CosmosInputHandler::default();
        input.set_keycode(CosmosInputs::ToggleQuestsUi, KeyCode::Tab);
        let mut app = App::new();
        app.insert_resource(quests)
            .insert_resource(Lang::<Quest>::new("en_us", vec![]))
            .insert_resource(DefaultFont(Handle::default()))
            .insert_resource(input)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, display_active_mission);
        let player = app.world_mut().spawn((LocalPlayer, ongoing, ActiveQuest(active))).id();
        app.update();
        (app, player)
    }

    fn visible_text(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect()
    }

    #[test]
    fn tracker_shows_all_steps_descriptions_and_numeric_progress_including_completed_counts() {
        let (mut app, _) = setup();
        let text = visible_text(&mut app);
        for expected in [
            "Mission",
            "Move",
            "Supplies",
            "Craft",
            "Board",
            "A mission with several simultaneous objectives.",
            "Move your ship in each direction.",
            "Prepare supplies before leaving.",
            "Fabricate the required parts.",
            "Enter your ship.",
            "1/3",
            "37/100",
            "20/20",
            "[Tab] All quests",
        ] {
            assert!(text.iter().any(|value| value == expected), "Missing {expected}");
        }
        assert!(!text.iter().any(|value| value == "0/0"));
        assert!(text.iter().filter(|value| value.as_str() == constants::CHECK.to_string()).count() >= 2);
        let (_, tint) = app
            .world_mut()
            .query::<(&Text, &TextColor)>()
            .iter(app.world())
            .find(|(text, _)| text.0 == "Craft")
            .unwrap();
        assert_eq!(tint.0, TRACKER_MUTED);
    }

    #[test]
    fn switching_primary_quest_keeps_the_replacement_visible_on_following_frames() {
        let (mut app, player) = setup();
        let second = {
            let mut quests = app.world_mut().resource_mut::<Registry<Quest>>();
            quests.register(Quest::new("Other mission", "The newly selected quest."));
            quests.from_id("Other mission").unwrap().clone()
        };
        let mut second = OngoingQuest::new(&second, Default::default(), 4);
        second.set_progress(2);
        let second_id = app.world_mut().get_mut::<OngoingQuests>(player).unwrap().start_quest(second);
        app.world_mut().entity_mut(player).insert(ActiveQuest(second_id));

        for _ in 0..4 {
            app.update();
            let text = visible_text(&mut app);
            assert!(text.iter().any(|text| text == "Other mission"));
            assert!(text.iter().any(|text| text == "2/4"));
            assert!(!text.iter().any(|text| text == "Mission"));
            assert_eq!(
                app.world_mut()
                    .query_filtered::<Entity, With<ActiveMissionDisplay>>()
                    .iter(app.world())
                    .count(),
                1
            );
        }
    }

    #[test]
    fn unrelated_active_quest_removals_do_not_hide_the_current_tracker() {
        let (mut app, player) = setup();
        let active = *app.world().get::<ActiveQuest>(player).unwrap();
        let other_player = app.world_mut().spawn(active).id();
        app.update();
        app.world_mut().entity_mut(other_player).remove::<ActiveQuest>();
        app.update();
        assert_eq!(app.world().get::<ActiveQuest>(player), Some(&active));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ActiveMissionDisplay>>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(visible_text(&mut app).iter().any(|text| text == "Mission"));
    }

    #[test]
    fn progress_refresh_preserves_scroll_and_untracking_removes_the_panel() {
        let (mut app, player) = setup();
        app.world_mut()
            .query_filtered::<&mut ScrollPosition, With<MissionContent>>()
            .single_mut(app.world_mut())
            .unwrap()
            .y = 42.0;
        let move_quest = app.world().resource::<Registry<Quest>>().from_id("Move").unwrap().clone();
        {
            let mut ongoing = app.world_mut().get_mut::<OngoingQuests>(player).unwrap();
            ongoing
                .iter_mut()
                .next()
                .unwrap()
                .subquests_mut()
                .unwrap()
                .get_quest_mut(&move_quest)
                .unwrap()
                .set_progress(58);
        }
        app.update();
        let text = visible_text(&mut app);
        assert!(text.iter().any(|text| text == "58/100"));
        assert!(!text.iter().any(|text| text == "37/100"));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ActiveMissionDisplay>>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query_filtered::<&ScrollPosition, With<MissionContent>>()
                .single(app.world())
                .unwrap()
                .y,
            42.0
        );
        app.world_mut().entity_mut(player).remove::<ActiveQuest>();
        app.update();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<ActiveMissionDisplay>>()
                .iter(app.world())
                .next()
                .is_none()
        );
    }
}
