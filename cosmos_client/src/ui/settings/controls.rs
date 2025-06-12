// //! Handles the rendering of the settings UI
//
// use bevy::{prelude::*, utils::hashbrown::HashMap};
// use cosmos_core::registry::{Registry, identifiable::Identifiable};
//
// use crate::{
//     input::inputs::CosmosInputHandler,
//     lang::Lang,
//     settings::{Setting, SettingCategory, SettingConstraint, SettingData},
//     ui::{
//         components::{
//             button::{ButtonEvent, ButtonStyles, CosmosButton},
//             text_input::{InputType, InputValue, TextInput},
//         },
//         reactivity::{BindValue, BindValues, ReactableFields, ReactableValue},
//     },
// };
//
// use super::super::{
//     UiSystemSet,
//     components::{
//         button::register_button,
//         scollable_container::ScrollBox,
//         slider::{Slider, SliderValue},
//     },
//     font::DefaultFont,
//     reactivity::add_reactable_type,
// };
//
// #[derive(Component)]
// /// Add this to a UI NodeBundle when you need a settings screen added to it
// pub struct NeedsControlsAdded;
//
// #[derive(Component)]
// struct SettingsMenu;
//
// fn create_settings_screen(
//     mut commands: Commands,
//     q_ui_root: Query<Entity, (Without<SettingsMenu>, With<NeedsControlsAdded>)>,
//     controls: Res<CosmosInputHandler>,
//     lang: Res<Lang<Setting>>,
//     mut q_style: Query<&mut Node, With<NeedsControlsAdded>>,
//     default_font: Res<DefaultFont>,
// ) {
//     let Ok(main_menu_root) = q_ui_root.get_single() else {
//         return;
//     };
//
//     let cool_blue = Srgba::hex("00FFFF").unwrap().into();
//
//     let blue_text = TextColor(cool_blue);
//     let text_style_large = TextFont {
//         font_size: 64.0,
//         font: default_font.0.clone(),
//         ..Default::default()
//     };
//
//     let text_style = TextFont {
//         font_size: 32.0,
//         font: default_font.0.clone(),
//         ..Default::default()
//     };
//     let text_style_small = TextFont {
//         font_size: 24.0,
//         font: default_font.0.clone(),
//         ..Default::default()
//     };
//
//     q_style
//         .get_mut(main_menu_root)
//         .expect("Attempted to insert settings menu into non-UI element")
//         .flex_direction = FlexDirection::Column;
//
//     commands.entity(main_menu_root).insert(SettingsMenu).with_children(|p| {
//         p.spawn((
//             Text::new("CONTROLS"),
//             text_style_large,
//             blue_text,
//             Node {
//                 margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(100.0), Val::Px(70.0)),
//                 align_self: AlignSelf::Center,
//                 ..Default::default()
//             },
//         ));
//
//         p.spawn((
//             Node {
//                 flex_grow: 1.0,
//                 margin: UiRect::new(Val::Percent(10.0), Val::Percent(10.0), Val::Px(0.0), Val::Px(0.0)),
//                 ..Default::default()
//             },
//             ScrollBox::default(),
//         ))
//         .with_children(|p| {
//             for (control, value) in controls.iter() {
//                 p.spawn((Text::new(format!("{:?} - {:?}", control, value)), text_style_small.clone()));
//             }
//         });
//
//         p.spawn(Node {
//             width: Val::Percent(100.0),
//             justify_content: JustifyContent::Center,
//             margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(70.0), Val::Px(30.0)),
//             column_gap: Val::Px(50.0),
//             ..Default::default()
//         })
//         .with_children(|p| {
//             p.spawn((
//                 BorderColor(cool_blue),
//                 Node {
//                     border: UiRect::all(Val::Px(2.0)),
//                     width: Val::Px(500.0),
//                     height: Val::Px(70.0),
//                     align_self: AlignSelf::Center,
//                     margin: UiRect::top(Val::Px(20.0)),
//                     ..Default::default()
//                 },
//                 CosmosButton::<ControlsCancelButtonEvent> {
//                     button_styles: Some(ButtonStyles {
//                         background_color: Srgba::hex("333333").unwrap().into(),
//                         hover_background_color: Srgba::hex("232323").unwrap().into(),
//                         press_background_color: Srgba::hex("111111").unwrap().into(),
//                         ..Default::default()
//                     }),
//                     text: Some(("Cancel".into(), text_style.clone(), Default::default())),
//                     ..Default::default()
//                 },
//             ));
//
//             p.spawn((
//                 BorderColor(cool_blue),
//                 Node {
//                     border: UiRect::all(Val::Px(2.0)),
//                     width: Val::Px(500.0),
//                     height: Val::Px(70.0),
//                     align_self: AlignSelf::Center,
//                     margin: UiRect::top(Val::Px(20.0)),
//                     ..Default::default()
//                 },
//                 CosmosButton::<ControlsDoneButtonEvent> {
//                     button_styles: Some(ButtonStyles {
//                         background_color: Srgba::hex("333333").unwrap().into(),
//                         hover_background_color: Srgba::hex("232323").unwrap().into(),
//                         press_background_color: Srgba::hex("111111").unwrap().into(),
//                         ..Default::default()
//                     }),
//                     text: Some(("Done".into(), text_style.clone(), Default::default())),
//                     ..Default::default()
//                 },
//             ));
//         });
//     });
// }
//
// fn done_clicked(mut settings: ResMut<Registry<Setting>>) {
//     // for written_setting in q_written_settings.iter() {
//     //     let setting = settings.from_numeric_id_mut(written_setting.setting_id);
//     //
//     //     match setting.data {
//     //         SettingData::I32(_) => {
//     //             let Ok(parsed) = written_setting.value.parse::<i32>() else {
//     //                 warn!("Invalid i32 - {}", written_setting.value);
//     //                 continue;
//     //             };
//     //
//     //             setting.data = SettingData::I32(parsed);
//     //         }
//     //         SettingData::String(_) => {
//     //             setting.data = SettingData::String(written_setting.value.clone());
//     //         }
//     //     }
//     //
//     //     info!("{setting:?}");
//     // }
// }
//
// #[derive(Event, Debug)]
// /// The cancel button was clicked on the settings menu
// ///
// /// The entity is the button's entity
// pub struct ControlsCancelButtonEvent(pub Entity);
//
// impl ButtonEvent for ControlsCancelButtonEvent {
//     fn create_event(e: Entity) -> Self {
//         Self(e)
//     }
// }
//
// #[derive(Event, Debug)]
// /// The done button was clicked on the settings menu
// ///
// /// The entity is the button's entity
// pub struct ControlsDoneButtonEvent(pub Entity);
//
// impl ButtonEvent for ControlsDoneButtonEvent {
//     fn create_event(e: Entity) -> Self {
//         Self(e)
//     }
// }
//
// #[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
// pub(super) enum ControlsMenuSet {
//     ControlsMenuInteractions,
// }
//
// pub(super) fn register(app: &mut App) {
//     register_button::<ControlsCancelButtonEvent>(app);
//     register_button::<ControlsDoneButtonEvent>(app);
//
//     app.add_systems(
//         Update,
//         (
//             create_settings_screen
//                 .in_set(UiSystemSet::DoUi)
//                 .before(ControlsMenuSet::ControlsMenuInteractions),
//             done_clicked
//                 .run_if(on_event::<ControlsDoneButtonEvent>)
//                 .in_set(ControlsMenuSet::ControlsMenuInteractions),
//         ),
//     );
// }
