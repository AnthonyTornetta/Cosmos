use bevy::{app::App, prelude::*, utils::hashbrown::HashMap};
use cosmos_core::registry::{identifiable::Identifiable, Registry};

use crate::{
    lang::Lang,
    settings::{Setting, SettingCategory, SettingData},
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::ScrollBundle,
            text_input::{InputType, InputValue, TextInput, TextInputBundle},
        },
        reactivity::{add_reactable_type, BindValue, BindValues, ReactableFields, ReactableValue},
    },
};

use super::{in_main_menu_state, MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet};

#[derive(Debug, Clone, PartialEq, Eq, Component)]
struct WrittenSetting {
    value: String,
    setting_id: u16,
}

impl ReactableValue for WrittenSetting {
    fn as_value(&self) -> String {
        self.value.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.value = new_value.to_owned()
    }
}

fn create_disconnect_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
    settings: Res<Registry<Setting>>,
    lang: Res<Lang<Setting>>,
) {
    let cool_blue = Color::hex("00FFFF").unwrap();

    let text_style_large = TextStyle {
        color: cool_blue,
        font_size: 64.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };
    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };
    let text_style_small = TextStyle {
        color: Color::WHITE,
        font_size: 24.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };
    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn(TextBundle {
            text: Text::from_section("SETTINGS", text_style_large.clone()),
            style: Style {
                margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(100.0), Val::Px(70.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            ..Default::default()
        });

        p.spawn(ScrollBundle {
            node_bundle: NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    margin: UiRect::new(Val::Percent(10.0), Val::Percent(10.0), Val::Px(0.0), Val::Px(0.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|p| {
            let mut categorized_settings: HashMap<SettingCategory, Vec<(&Setting, &str)>> = HashMap::default();
            for setting in settings.iter() {
                let unlocalized_name = setting.unlocalized_name();

                categorized_settings
                    .entry(setting.setting_category)
                    .or_default()
                    .push((setting, lang.get_name_from_id(unlocalized_name).unwrap_or(unlocalized_name)));
            }

            let mut categorized_settings = categorized_settings
                .into_iter()
                .collect::<Vec<(SettingCategory, Vec<(&Setting, &str)>)>>();

            categorized_settings.sort_by(|a, b| a.0.cmp(&b.0));

            for (category, mut settings) in categorized_settings {
                let category_display_name = match category {
                    SettingCategory::Graphics => "Graphics",
                    SettingCategory::Mouse => "Mouse",
                };

                p.spawn(TextBundle {
                    text: Text::from_section(category_display_name, text_style.clone()),
                    style: Style {
                        margin: UiRect::bottom(Val::Px(20.0)),
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                });

                settings.sort_by(|(_, x), (_, y)| x.to_lowercase().cmp(&y.to_lowercase()));

                for (setting, display_name) in settings {
                    p.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(20.0),
                            margin: UiRect::bottom(Val::Px(20.0)),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|p| {
                        let input_value = match &setting.data {
                            SettingData::F32(f) => format!("{f}"),
                            SettingData::String(s) => s.to_owned(),
                        };

                        let data_ent = p
                            .spawn((
                                WrittenSetting {
                                    setting_id: setting.id(),
                                    value: input_value.clone(),
                                },
                                TextBundle {
                                    text: Text::from_section(display_name, text_style.clone()),
                                    style: Style {
                                        width: Val::Px(500.0),
                                        align_self: AlignSelf::Center,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            ))
                            .id();

                        p.spawn((
                            BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Value)),
                            TextInputBundle {
                                text_input: TextInput {
                                    style: text_style_small.clone(),
                                    input_type: match &setting.data {
                                        SettingData::F32(_) => InputType::Decimal {
                                            min: f64::MIN,
                                            max: f64::MAX,
                                        },
                                        SettingData::String(_) => InputType::Text { max_length: None },
                                    },
                                    ..Default::default()
                                },
                                value: InputValue::new(input_value),
                                node_bundle: NodeBundle {
                                    border_color: Color::hex("555555").unwrap().into(),
                                    background_color: Color::hex("111111").unwrap().into(),
                                    style: Style {
                                        border: UiRect::all(Val::Px(2.0)),
                                        width: Val::Px(150.0),
                                        height: Val::Px(45.0),
                                        align_self: AlignSelf::Center,
                                        padding: UiRect {
                                            top: Val::Px(4.0),
                                            bottom: Val::Px(4.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            },
                        ));
                    });
                }
            }
        });

        p.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(70.0), Val::Px(30.0)),
                column_gap: Val::Px(50.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn(ButtonBundle::<CancelButtonEvent> {
                node_bundle: NodeBundle {
                    border_color: cool_blue.into(),
                    style: Style {
                        border: UiRect::all(Val::Px(2.0)),
                        width: Val::Px(500.0),
                        height: Val::Px(70.0),
                        align_self: AlignSelf::Center,
                        margin: UiRect::top(Val::Px(20.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                button: Button {
                    button_styles: Some(ButtonStyles {
                        background_color: Color::hex("333333").unwrap(),
                        hover_background_color: Color::hex("232323").unwrap(),
                        press_background_color: Color::hex("111111").unwrap(),
                        ..Default::default()
                    }),
                    text: Some(("Cancel".into(), text_style.clone())),
                    ..Default::default()
                },
            });

            p.spawn(ButtonBundle::<DoneButtonEvent> {
                node_bundle: NodeBundle {
                    border_color: cool_blue.into(),
                    style: Style {
                        border: UiRect::all(Val::Px(2.0)),
                        width: Val::Px(500.0),
                        height: Val::Px(70.0),
                        align_self: AlignSelf::Center,
                        margin: UiRect::top(Val::Px(20.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                button: Button {
                    button_styles: Some(ButtonStyles {
                        background_color: Color::hex("333333").unwrap(),
                        hover_background_color: Color::hex("232323").unwrap(),
                        press_background_color: Color::hex("111111").unwrap(),
                        ..Default::default()
                    }),
                    text: Some(("Done".into(), text_style.clone())),
                    ..Default::default()
                },
            });
        });
    });
}

#[derive(Default, Event, Debug)]
struct CancelButtonEvent;

impl ButtonEvent for CancelButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Default, Event, Debug)]
struct DoneButtonEvent;

impl ButtonEvent for DoneButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn cancel_clicked(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::TitleScreen;
}

fn done_clicked(mut mms: ResMut<MainMenuSubState>, mut settings: ResMut<Registry<Setting>>, q_written_settings: Query<&WrittenSetting>) {
    for written_setting in q_written_settings.iter() {
        let setting = settings.from_numeric_id_mut(written_setting.setting_id);

        match setting.data {
            SettingData::F32(_) => {
                let Ok(parsed) = written_setting.value.parse::<f32>() else {
                    continue;
                };

                setting.data = SettingData::F32(parsed);
            }
            SettingData::String(_) => {
                setting.data = SettingData::String(written_setting.value.clone());
            }
        }
    }

    *mms = MainMenuSubState::TitleScreen;
}

pub(super) fn register(app: &mut App) {
    register_button::<CancelButtonEvent>(app);
    register_button::<DoneButtonEvent>(app);

    add_reactable_type::<WrittenSetting>(app);

    app.add_systems(
        Update,
        create_disconnect_screen
            .run_if(in_main_menu_state(MainMenuSubState::Settings))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    )
    .add_systems(
        Update,
        cancel_clicked
            .run_if(on_event::<CancelButtonEvent>())
            .run_if(in_main_menu_state(MainMenuSubState::Settings))
            .in_set(MainMenuSystemSet::UpdateMenu),
    )
    .add_systems(
        Update,
        done_clicked
            .run_if(on_event::<DoneButtonEvent>())
            .run_if(in_main_menu_state(MainMenuSubState::Settings))
            .in_set(MainMenuSystemSet::UpdateMenu),
    );
}
