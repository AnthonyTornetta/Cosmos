//! Handles the rendering of the settings UI

use bevy::{prelude::*, utils::hashbrown::HashMap};
use cosmos_core::registry::{identifiable::Identifiable, Registry};

use crate::{
    lang::Lang,
    settings::{Setting, SettingCategory, SettingConstraint, SettingData},
    ui::{
        components::{
            button::{Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::ScrollBundle,
            text_input::{InputType, InputValue, TextInput, TextInputBundle},
        },
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue},
    },
};

use super::{
    components::{
        button::register_button,
        slider::{Slider, SliderBundle, SliderValue},
    },
    reactivity::add_reactable_type,
    UiSystemSet,
};

#[derive(Component)]
/// Add this to a UI NodeBundle when you need a settings screen added to it
pub struct NeedsSettingsAdded;

#[derive(Debug, Reflect, Clone, PartialEq, Eq, Component)]
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

#[derive(Component)]
struct SettingsMenu;

fn create_settings_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_ui_root: Query<Entity, (Without<SettingsMenu>, With<NeedsSettingsAdded>)>,
    settings: Res<Registry<Setting>>,
    lang: Res<Lang<Setting>>,
    mut q_style: Query<&mut Style, With<NeedsSettingsAdded>>,
) {
    let Ok(main_menu_root) = q_ui_root.get_single() else {
        return;
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap().into();

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

    q_style
        .get_mut(main_menu_root)
        .expect("Attempted to insert settings menu into non-UI element")
        .flex_direction = FlexDirection::Column;

    commands.entity(main_menu_root).insert(SettingsMenu).with_children(|p| {
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
                            SettingData::I32(f) => format!("{f}"),
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

                        match setting.constraint {
                            None => {
                                p.spawn((
                                    BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Value)),
                                    TextInputBundle {
                                        text_input: TextInput {
                                            style: text_style_small.clone(),
                                            input_type: match &setting.data {
                                                SettingData::I32(_) => InputType::Integer {
                                                    min: i32::MIN as i64,
                                                    max: i32::MAX as i64,
                                                },
                                                SettingData::String(_) => InputType::Text { max_length: None },
                                            },
                                            ..Default::default()
                                        },
                                        value: InputValue::new(input_value),
                                        node_bundle: NodeBundle {
                                            border_color: Srgba::hex("555555").unwrap().into(),
                                            background_color: Srgba::hex("111111").unwrap().into(),
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
                            }
                            Some(SettingConstraint::I32 { min, max }) => {
                                let SettingData::I32(value) = setting.data else {
                                    panic!("Cannot have f32 constraint for non-f32 value!");
                                };

                                p.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Px(300.0),
                                        justify_content: JustifyContent::SpaceBetween,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with_children(|p| {
                                    p.spawn((
                                        BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Value)),
                                        SliderBundle {
                                            slider_value: SliderValue::new(value as i64),
                                            slider: Slider {
                                                min: min as i64,
                                                max: max as i64,
                                                background_color: Srgba::hex("111111").unwrap().into(),
                                                foreground_color: Srgba::hex("555555").unwrap().into(),
                                                ..Default::default()
                                            },
                                            node_bundle: NodeBundle {
                                                style: Style {
                                                    width: Val::Px(200.0),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                    ));

                                    p.spawn((
                                        BindValues::single(BindValue::<WrittenSetting>::new(
                                            data_ent,
                                            ReactableFields::Text { section: 0 },
                                        )),
                                        TextBundle {
                                            text: Text::from_section(format!("{value}"), text_style_small.clone()),
                                            ..Default::default()
                                        },
                                    ));
                                });
                            }
                        }
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
            p.spawn(ButtonBundle::<SettingsCancelButtonEvent> {
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
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Cancel".into(), text_style.clone())),
                    ..Default::default()
                },
            });

            p.spawn(ButtonBundle::<SettingsDoneButtonEvent> {
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
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Done".into(), text_style.clone())),
                    ..Default::default()
                },
            });
        });
    });
}

fn done_clicked(mut settings: ResMut<Registry<Setting>>, q_written_settings: Query<&WrittenSetting>) {
    println!(":D");
    for written_setting in q_written_settings.iter() {
        let setting = settings.from_numeric_id_mut(written_setting.setting_id);

        match setting.data {
            SettingData::I32(_) => {
                let Ok(parsed) = written_setting.value.parse::<i32>() else {
                    warn!("Invalid i32 - {}", written_setting.value);
                    continue;
                };

                setting.data = SettingData::I32(parsed);
            }
            SettingData::String(_) => {
                setting.data = SettingData::String(written_setting.value.clone());
            }
        }

        info!("{setting:?}");
    }
}

#[derive(Event, Debug)]
/// The cancel button was clicked on the settings menu
///
/// The entity is the button's entity
pub struct SettingsCancelButtonEvent(pub Entity);

impl ButtonEvent for SettingsCancelButtonEvent {
    fn create_event(e: Entity) -> Self {
        Self(e)
    }
}

#[derive(Event, Debug)]
/// The done button was clicked on the settings menu
///
/// The entity is the button's entity
pub struct SettingsDoneButtonEvent(pub Entity);

impl ButtonEvent for SettingsDoneButtonEvent {
    fn create_event(e: Entity) -> Self {
        Self(e)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum SettingsMenuSet {
    SettingsMenuInteractions,
}

pub(super) fn register(app: &mut App) {
    register_button::<SettingsCancelButtonEvent>(app);
    register_button::<SettingsDoneButtonEvent>(app);

    add_reactable_type::<WrittenSetting>(app);

    app.add_systems(
        Update,
        (
            create_settings_screen
                .in_set(UiSystemSet::DoUi)
                .before(SettingsMenuSet::SettingsMenuInteractions),
            done_clicked
                .run_if(on_event::<SettingsDoneButtonEvent>())
                .in_set(SettingsMenuSet::SettingsMenuInteractions),
        ),
    )
    .register_type::<WrittenSetting>();
}
