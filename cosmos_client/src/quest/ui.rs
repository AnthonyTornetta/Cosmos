use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    quest::{OngoingQuest, OngoingQuests, Quest},
    registry::Registry,
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{
        components::{
            show_cursor::{no_open_menus, ShowCursor},
            window::GuiWindow,
        },
        font::DefaultFont,
        OpenMenu,
    },
};

#[derive(Component)]
struct QuestUi;

fn open_quest_ui(
    input_checker: InputChecker,
    mut commands: Commands,
    default_font: Res<DefaultFont>,
    q_quests_ui: Query<Entity, With<QuestUi>>,
    q_quests: Query<&OngoingQuests, With<LocalPlayer>>,
    quests: Res<Registry<Quest>>,
    lang: Res<Lang<Quest>>,
) {
    if !input_checker.check_just_pressed(CosmosInputs::ToggleQuestsUi) {
        return;
    }

    if let Ok(ent) = q_quests_ui.get_single() {
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
            let ongoing_quests = q_quests.get_single().map(|x| x.iter().collect::<Vec<_>>()).unwrap_or_default();

            if ongoing_quests.is_empty() {
                p.spawn((Text::new("No Active Missions."), font.clone()));
                p.spawn((Text::new("Hopefully a Merchant will give you one soon."), font.clone()));
                return;
            }

            for quest in ongoing_quests {
                quest_node(p, quest, &quests, &lang, font_small.clone());
            }
        });
}

fn quest_node(commands: &mut ChildBuilder, ongoing: &OngoingQuest, quests: &Registry<Quest>, lang: &Lang<Quest>, font: TextFont) {
    let Some(quest) = quests.try_from_numeric_id(ongoing.quest_id) else {
        return;
    };

    commands
        .spawn((
            Name::new("Quest UI Entry"),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(70.0),
                border: UiRect::vertical(Val::Px(2.0)),
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            BorderColor(css::LIGHT_GREY.into()),
        ))
        .with_children(|p| {
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
                    Text::new(format!("{}", lang.get_name_or_unlocalized(quest))),
                ));
                p.spawn((Name::new("Quest Desc"), font.clone(), Text::new(format!("{}", quest.description))));
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
    app.add_systems(
        Update,
        open_quest_ui
            .run_if(no_open_menus.or(any_with_component::<QuestUi>))
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between),
    );
}
