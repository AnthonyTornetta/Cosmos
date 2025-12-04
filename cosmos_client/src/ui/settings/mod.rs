//! Handles the rendering of the settings UI

use bevy::{color::palettes::css, platform::collections::HashMap, prelude::*};
use cosmos_core::registry::{Registry, identifiable::Identifiable};

use crate::{
    input::inputs::{ControlType, CosmosInputHandler, CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    settings::{Setting, SettingCategory, SettingConstraint, SettingData},
    ui::{
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            scollable_container::ScrollbarStyle,
            text_input::{InputType, InputValue, TextInput},
        },
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue},
    },
};

use super::{
    UiSystemSet,
    components::{
        scollable_container::ScrollBox,
        slider::{Slider, SliderValue},
        tabbed_view::{Tab, TabbedView},
    },
    font::DefaultFont,
    reactivity::add_reactable_type,
};

#[derive(Component)]
/// Add this to a UI NodeBundle when you need a settings screen added to it
pub struct NeedsSettingsAdded;

#[derive(Debug, Reflect, Clone, Component)]
struct SettingControlValue {
    input: CosmosInputs,
    value: Option<ControlType>,
}

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

#[derive(Component)]
struct ListeningNextInput;

fn create_settings_screen(
    mut commands: Commands,
    q_ui_root: Query<Entity, (Without<SettingsMenu>, With<NeedsSettingsAdded>)>,
    settings: Res<Registry<Setting>>,
    lang: Res<Lang<Setting>>,
    mut q_style: Query<&mut Node, With<NeedsSettingsAdded>>,
    default_font: Res<DefaultFont>,
    controls: Res<CosmosInputHandler>,
) {
    let Ok(main_menu_root) = q_ui_root.single() else {
        return;
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap();

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
            Node {
                width: Val::Px(1000.0),
                margin: UiRect {
                    left: Val::Auto,
                    right: Val::Auto,
                    top: Val::Px(50.0),
                    ..Default::default()
                },
                flex_grow: 1.0,
                scrollbar_width: 10.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..Default::default()
            },
            ScrollbarStyle { ..Default::default() },
            ScrollPosition::default(),
            Pickable::default(),
            // TabbedView {
            //     body_styles: Node {
            //         margin: UiRect::top(Val::Px(20.0)),
            //         ..Default::default() },
            //     view_background: Color::NONE.into(),
            //     ..Default::default()
            // },
        ))
        .with_children(|p| {
            create_general_tab(&settings, &lang, &text_style, &text_style_small, p);
            create_controls_tab(&controls, &text_style, &text_style_small, p);
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
                BorderColor::all(cool_blue),
                Node {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
                CosmosButton {
                    button_styles: Some(ButtonStyles {
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Cancel".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ))
            .observe(
                |ev: On<ButtonEvent>, mut evw_settings_cancel: MessageWriter<SettingsCancelButtonMessage>| {
                    evw_settings_cancel.write(SettingsCancelButtonMessage(ev.0));
                },
            );

            p.spawn((
                BorderColor::all(cool_blue),
                Node {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
                CosmosButton {
                    button_styles: Some(ButtonStyles {
                        background_color: Srgba::hex("333333").unwrap().into(),
                        hover_background_color: Srgba::hex("232323").unwrap().into(),
                        press_background_color: Srgba::hex("111111").unwrap().into(),
                        ..Default::default()
                    }),
                    text: Some(("Done".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ))
            .observe(done_clicked);
        });
    });
}

fn create_general_tab(
    settings: &Registry<Setting>,
    lang: &Lang<Setting>,
    text_style: &TextFont,
    text_style_small: &TextFont,
    p: &mut ChildSpawnerCommands,
) {
    p.spawn((
        Tab::new("General"),
        BackgroundColor(css::RED.into()),
        Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::new(Val::Percent(10.0), Val::Percent(10.0), Val::Px(0.0), Val::Px(0.0)),
            ..Default::default()
        },
        // ScrollBox::default(),
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
                SettingCategory::Audio => "Audio",
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
                                BorderColor::all(Srgba::hex("555555").unwrap()),
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
                                    BindValues::single(BindValue::<WrittenSetting>::new(data_ent, ReactableFields::Text { section: 0 })),
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
}

fn create_controls_tab(controls: &CosmosInputHandler, text_style: &TextFont, text_style_small: &TextFont, p: &mut ChildSpawnerCommands) {
    p.spawn((
        Tab::new("Controls"),
        Node {
            margin: UiRect::new(Val::Percent(10.0), Val::Percent(10.0), Val::Px(0.0), Val::Px(0.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        // ScrollBox::default(),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("Controls"),
            text_style.clone(),
            Node {
                margin: UiRect::vertical(Val::Px(20.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        ));

        let mut inputs = controls.iter().filter(|(x, _)| **x != CosmosInputs::Pause).collect::<Vec<_>>();
        inputs.sort_by_key(|x| *x.0);

        for (input, mapping) in inputs {
            p.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(20.0),
                margin: UiRect::bottom(Val::Px(20.0)),
                ..Default::default()
            })
            .with_children(|p| {
                p.spawn((
                    Text::new(format!("{input:?}")),
                    text_style.clone(),
                    Node {
                        flex_grow: 1.0,
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                ));

                p.spawn((
                    CosmosButton {
                        text: Some(("".to_owned(), text_style_small.clone(), Default::default())),
                        ..Default::default()
                    },
                    SettingControlValue {
                        input: *input,
                        value: *mapping,
                    },
                    BorderColor::all(Srgba::hex("555555").unwrap()),
                    BackgroundColor(Srgba::hex("111111").unwrap().into()),
                    Node {
                        border: UiRect::all(Val::Px(2.0)),
                        width: Val::Px(300.0),
                        height: Val::Px(45.0),
                        align_self: AlignSelf::Center,
                        padding: UiRect {
                            top: Val::Px(4.0),
                            bottom: Val::Px(4.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .observe(click_settings_button);
            });
        }
    });
}

fn click_settings_button(
    ev: On<ButtonEvent>,
    mut commands: Commands,
    q_next_input: Query<(), With<ListeningNextInput>>,
    mut q_button: Query<&mut CosmosButton>,
    mut clicked_this_frame: RemovedComponents<ListeningNextInput>,
) {
    if !q_next_input.is_empty() {
        return;
    }

    if clicked_this_frame.read().any(|x| x == ev.0) {
        // This means that setting the control to `mouse 1` won't immediately try to re-set it.
        return;
    }

    if let Ok(mut btn) = q_button.get_mut(ev.0) {
        let cur_val = btn.text.as_mut().unwrap();
        cur_val.0 = format!("> {} <", cur_val.0);
    }
    commands.entity(ev.0).insert(ListeningNextInput);
}

fn listen_for_inputs(
    mut q_listening: Query<(Entity, &mut SettingControlValue), With<ListeningNextInput>>,
    mut commands: Commands,
    inputs: InputChecker,
) {
    if inputs.check_pressed(CosmosInputs::Pause) {
        for (ent, mut settings_val) in q_listening.iter_mut() {
            settings_val.value = None;
            commands.entity(ent).remove::<ListeningNextInput>();
        }
        return;
    }
    for (ent, mut settings_val) in q_listening.iter_mut() {
        if let Some(key) = inputs.any_key_released() {
            settings_val.value = Some(ControlType::Key(key));
            commands.entity(ent).remove::<ListeningNextInput>();
        } else if let Some(mouse) = inputs.any_mouse_released() {
            settings_val.value = Some(ControlType::Mouse(mouse));
            commands.entity(ent).remove::<ListeningNextInput>();
        }
    }
}

fn on_change_setting_value(mut q_changed_setting: Query<(&mut CosmosButton, &SettingControlValue), Changed<SettingControlValue>>) {
    for (mut btn, value) in q_changed_setting.iter_mut() {
        btn.text.as_mut().unwrap().0 = match value.value {
            None => "[None]".to_owned(),
            Some(c) => c.to_string(),
        }
    }
}

fn done_clicked(
    ev: On<ButtonEvent>,
    mut settings: ResMut<Registry<Setting>>,
    q_written_settings: Query<&WrittenSetting>,
    q_setting: Query<&SettingControlValue>,
    mut inputs: ResMut<CosmosInputHandler>,
    mut evw_done: MessageWriter<SettingsDoneButtonMessage>,
) {
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
    }

    for control in q_setting.iter() {
        match control.value {
            None => {
                inputs.remove_control(control.input);
            }
            Some(ControlType::Mouse(m)) => {
                inputs.set_mouse_button(control.input, m);
            }
            Some(ControlType::Key(k)) => {
                inputs.set_keycode(control.input, k);
            }
        }
    }

    evw_done.write(SettingsDoneButtonMessage(ev.0));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum SettingsMenuSet {
    SettingsMenuInteractions,
}

#[derive(Message, Debug)]
/// Sent when the Settings Cancel button is clicked
pub struct SettingsCancelButtonMessage(pub Entity);
#[derive(Message, Debug)]
/// Sent when the Settings Done button is clicked
pub struct SettingsDoneButtonMessage(pub Entity);

fn make_it(mut commands: Commands, q_n: Query<Entity, (With<Node>, Without<Pickable>)>) {
    for e in q_n.iter() {
        commands.entity(e).insert(Pickable::default());
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<WrittenSetting>(app);

    app.add_systems(
        Update,
        (
            (create_settings_screen, make_it)
                .chain()
                .in_set(UiSystemSet::DoUi)
                .before(SettingsMenuSet::SettingsMenuInteractions),
            (listen_for_inputs, on_change_setting_value)
                .chain()
                .in_set(SettingsMenuSet::SettingsMenuInteractions),
        ),
    )
    .register_type::<WrittenSetting>()
    .add_message::<SettingsDoneButtonMessage>()
    .add_message::<SettingsCancelButtonMessage>()
    // Spawn your entity here, e.g. a `Mesh3d`.
    // When dragged, mutate the `Transform` component on the dragged target entity:
    .add_observer(|drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>| {
        info!("DRAG {:?}!", drag.entity);
    })
    .add_observer(|click: On<Pointer<Click>>, mut commands: Commands| {
        info!("click {:?}!", click.entity);
    })
    .add_observer(|over: On<Pointer<Over>>| {
        info!("OVER {:?}!", over.entity);
    });
}
