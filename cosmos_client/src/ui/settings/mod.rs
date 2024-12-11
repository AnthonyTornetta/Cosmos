//! Handles the rendering of the settings UI

use bevy::{prelude::*, utils::hashbrown::HashMap};
use cosmos_core::registry::{identifiable::Identifiable, Registry};

use crate::{
    lang::Lang,
    settings::{Setting, SettingCategory, SettingConstraint, SettingData},
    ui::{
        components::{
            button::{Button, ButtonEvent, ButtonStyles},
            text_input::{InputType, InputValue, TextInput},
        },
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue},
    },
};

use super::{
    components::{
        button::register_button,
        scollable_container::ScrollBox,
        slider::{Slider, SliderValue},
    },
    font::DefaultFont,
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
    q_ui_root: Query<Entity, (Without<SettingsMenu>, With<NeedsSettingsAdded>)>,
    settings: Res<Registry<Setting>>,
    lang: Res<Lang<Setting>>,
    mut q_style: Query<&mut Node, With<NeedsSettingsAdded>>,
    default_font: Res<DefaultFont>,
) {
    let Ok(main_menu_root) = q_ui_root.get_single() else {
        return;
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap().into();

    let blue_text = TextColor(cool_blue);
    let text_style_large = TextFont {
        font_size: 64.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let text_style = TextFont {
        font_size: 32.0,
        font: default_font.0.clone(),
        ..Default::default()
    };
    let text_style_small = TextFont {
        font_size: 24.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    q_style
        .get_mut(main_menu_root)
        .expect("Attempted to insert settings menu into non-UI element")
        .flex_direction = FlexDirection::Column;

    commands.entity(main_menu_root).insert(SettingsMenu).with_children(|p| {
        p.spawn((
            Text::new("SETTINGS"),
            text_style_large,
            blue_text,
            Node {
                margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(100.0), Val::Px(70.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        ));

        p.spawn((
            Node {
                flex_grow: 1.0,
                margin: UiRect::new(Val::Percent(10.0), Val::Percent(10.0), Val::Px(0.0), Val::Px(0.0)),
                ..Default::default()
            },
            ScrollBox::default(),
        ))
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

                p.spawn((
                    Text::new(category_display_name),
                    text_style.clone(),
                    Node {
                        margin: UiRect::bottom(Val::Px(20.0)),
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                ));

                settings.sort_by(|(_, x), (_, y)| x.to_lowercase().cmp(&y.to_lowercase()));

                for (setting, display_name) in settings {
                    p.spawn(Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(20.0),
                        margin: UiRect::bottom(Val::Px(20.0)),
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
                                Text::new(display_name),
                                text_style.clone(),
                                Node {
                                    width: Val::Px(500.0),
                                    align_self: AlignSelf::Center,
                                    ..Default::default()
                                },
                            ))
                            .id();

                        match setting.constraint {
                            None => {
                                p.spawn((
                                    BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Value)),
                                    text_style_small.clone(),
                                    TextInput {
                                        input_type: match &setting.data {
                                            SettingData::I32(_) => InputType::Integer {
                                                min: i32::MIN as i64,
                                                max: i32::MAX as i64,
                                            },
                                            SettingData::String(_) => InputType::Text { max_length: None },
                                        },
                                        ..Default::default()
                                    },
                                    InputValue::new(input_value),
                                    BorderColor(Srgba::hex("555555").unwrap().into()),
                                    BackgroundColor(Srgba::hex("111111").unwrap().into()),
                                    Node {
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
                                ));
                            }
                            Some(SettingConstraint::I32 { min, max }) => {
                                let SettingData::I32(value) = setting.data else {
                                    panic!("Cannot have f32 constraint for non-f32 value!");
                                };

                                p.spawn(Node {
                                    width: Val::Px(300.0),
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..Default::default()
                                })
                                .with_children(|p| {
                                    p.spawn((
                                        BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Value)),
                                        SliderValue::new(value as i64),
                                        Slider {
                                            min: min as i64,
                                            max: max as i64,
                                            background_color: Srgba::hex("111111").unwrap().into(),
                                            foreground_color: Srgba::hex("555555").unwrap().into(),
                                            ..Default::default()
                                        },
                                        Node {
                                            width: Val::Px(200.0),
                                            ..Default::default()
                                        },
                                    ));

                                    p.spawn((
                                        BindValues::single(BindValue::<WrittenSetting>::new(
                                            data_ent,
                                            ReactableFields::Text { section: 0 },
                                        )),
                                        Text::new(format!("{value}")),
                                        text_style_small.clone(),
                                    ));
                                });
                            }
                        }
                    });
                }
            }
        });

        p.spawn(Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(70.0), Val::Px(30.0)),
            column_gap: Val::Px(50.0),
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn((
                BorderColor(cool_blue),
                Node {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
                Button::<SettingsCancelButtonEvent> {
                    button_styles: Some(ButtonStyles {
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Cancel".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ));

            p.spawn((
                BorderColor(cool_blue),
                Node {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
                Button::<SettingsDoneButtonEvent> {
                    button_styles: Some(ButtonStyles {
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Done".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ));
        });
    });
}

fn done_clicked(mut settings: ResMut<Registry<Setting>>, q_written_settings: Query<&WrittenSetting>) {
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
                .run_if(on_event::<SettingsDoneButtonEvent>)
                .in_set(SettingsMenuSet::SettingsMenuInteractions),
        ),
    )
    .register_type::<WrittenSetting>();
}
