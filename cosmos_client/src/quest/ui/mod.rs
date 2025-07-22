mod hud;

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::client::LocalPlayer,
    quest::{ActiveQuest, OngoingQuest, OngoingQuestId, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            button::{ButtonEvent, CosmosButton, register_button},
            scollable_container::ScrollBox,
            show_cursor::{ShowCursor, no_open_menus},
            window::GuiWindow,
        },
        font::DefaultFont,
    },
};

#[derive(Component)]
struct QuestUi;

fn open_quest_ui(
    input_checker: InputChecker,
    mut commands: Commands,
    default_font: Res<DefaultFont>,
    q_quests_ui: Query<Entity, With<QuestUi>>,
    q_quests: Query<(&OngoingQuests, Option<&ActiveQuest>), With<LocalPlayer>>,
    quests: Res<Registry<Quest>>,
    lang: Res<Lang<Quest>>,
) {
    if !input_checker.check_just_pressed(CosmosInputs::ToggleQuestsUi) {
        return;
    }

    if let Ok(ent) = q_quests_ui.single() {
        commands.entity(ent).insert(NeedsDespawned);
        return;
    }

    let font = TextFont {
        font: default_font.0.clone_weak(),
        font_size: 24.0,
        ..Default::default()
    };

    let font_small = TextFont {
        font: default_font.0.clone_weak(),
        font_size: 20.0,
        ..Default::default()
    };

    commands
        .spawn((
            Name::new("Ongoing Missions UI"),
            QuestUi,
            OpenMenu::new(0),
            ShowCursor,
            GuiWindow {
                title: "Ongoing Missions".into(),
                body_styles: Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
            Node {
                margin: UiRect::all(Val::Auto),
                position_type: PositionType::Absolute,
                width: Val::Px(600.0),
                height: Val::Px(800.0),
                ..Default::default()
            },
        ))
        .with_children(|p| {
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
                ScrollBox { ..Default::default() },
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

#[derive(Event, Debug)]
struct ToggleActiveClicked(Entity);

impl ButtonEvent for ToggleActiveClicked {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

fn on_toggle_active(
    mut commands: Commands,
    mut q_selected_quest: Query<Entity, With<LocalPlayer>>,
    mut evr_toggle_active: EventReader<ToggleActiveClicked>,
    mut q_active: Query<(Entity, &mut BorderColor), With<ActiveQuestUi>>,
    mut q_inactive: Query<(Entity, &QuestComp, &mut BorderColor), Without<ActiveQuestUi>>,
) {
    for ev in evr_toggle_active.read() {
        let Ok(player_ent) = q_selected_quest.single_mut() else {
            continue;
        };

        if let Ok((ent, mut bc)) = q_active.single_mut() {
            commands.entity(ent).remove::<ActiveQuestUi>();
            commands.entity(player_ent).remove::<ActiveQuest>();
            bc.0 = css::LIGHT_GREY.into();
        }

        let Ok((ui_ent, q, mut bc)) = q_inactive.get_mut(ev.0) else {
            commands.entity(player_ent).remove::<ActiveQuest>();
            continue;
        };

        bc.0 = css::AQUA.into();
        commands.entity(ui_ent).insert(ActiveQuestUi);
        commands.entity(player_ent).insert(ActiveQuest(q.0));
    }
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
        CosmosButton::<ToggleActiveClicked> { ..Default::default() },
        Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(70.0),
            border: UiRect::vertical(Val::Px(2.0)),
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        QuestComp(ongoing.ongoing_id()),
        BorderColor(if active { css::AQUA.into() } else { css::LIGHT_GREY.into() }),
    ));
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
            p.spawn((font.clone(), Name::new("Details"), Text::new("Details")));

            if let Some(payment) = ongoing.details.payout {
                p.spawn((
                    font.clone(),
                    Name::new("Quest Reward"),
                    TextColor(css::LIGHT_GREEN.into()),
                    Text::new(format!("+${payment}")),
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
    hud::register(app);

    register_button::<ToggleActiveClicked>(app);

    app.add_systems(
        Update,
        (open_quest_ui, on_toggle_active.in_set(UiSystemSet::FinishUi))
            .run_if(no_open_menus.or(any_with_component::<QuestUi>))
            .run_if(in_state(GameState::Playing)),
    );
}
