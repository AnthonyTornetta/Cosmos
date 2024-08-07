//! A UI component that is used to select a number between a range of values using a slider.
//!
//! Similar to the HTML `input type="range"`.use std::ops::Range;

use bevy::{
    app::{App, Update},
    color::{palettes::css, Color},
    core::Name,
    ecs::{
        bundle::Bundle,
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        query::{Added, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
        world::Ref,
    },
    hierarchy::BuildChildren,
    log::info,
    reflect::Reflect,
    transform::components::GlobalTransform,
    ui::{node_bundles::NodeBundle, BackgroundColor, Interaction, Node, PositionType, Style, UiRect, UiScale, Val},
    window::{PrimaryWindow, Window},
};

use crate::ui::UiSystemSet;

use super::Disabled;

#[derive(Component, Debug, Reflect)]
/// A UI component that is used to select a number between a range of values using a slider.
///
/// Similar to the HTML `input type="range"`.
pub struct Slider {
    /// Optional styles to further customize the slider
    pub slider_styles: Option<SliderStyles>,
    /// The minimum value you can slide to
    pub min: i64,
    /// The maximum value you can slide to
    pub max: i64,
    /// The color of the background bar
    pub background_color: Color,
    /// The color of the bar that represents % filled
    pub foreground_color: Color,
    /// The color of the square the user clicks to drag the bar around
    pub square_color: Color,
    /// The height the slider should be up its creation in px
    pub height: f32,
}

#[derive(Reflect, Component, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// The value the slider curently has selected
pub struct SliderValue(i64);

impl SliderValue {
    /// Creates a SliderValue with this initial value
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// Gets the value currently selected
    pub fn value(&self) -> i64 {
        self.0
    }

    /// Sets the value currently selected
    ///
    /// Updating this will change the UI
    pub fn set_value(&mut self, new_val: i64) {
        self.0 = new_val;
    }
}

#[derive(Default, Debug, Reflect)]
/// Styles to further customize the slider
pub struct SliderStyles {
    /// The color of the background bar
    pub hover_background_color: Color,
    /// The color of the bar that represents % filled when the slider is pressed
    pub hover_foreground_color: Color,

    /// The color of the background bar
    pub press_background_color: Color,
    /// The color of the bar that represents % filled when the slider is pressed
    pub press_foreground_color: Color,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            background_color: css::RED.into(),
            foreground_color: css::GRAY.into(),
            square_color: css::AQUAMARINE.into(),
            min: 0,
            max: 100,
            slider_styles: Default::default(),
            height: 10.0,
        }
    }
}

#[derive(Debug, Bundle, Default)]
/// A UI component that is used to select a number between a range of values using a slider.
///
/// Similar to the HTML `input type="range"`.
pub struct SliderBundle {
    /// The node bundle that will be used with the TextInput
    pub node_bundle: NodeBundle,
    /// The slider component
    pub slider: Slider,
    /// The value the slider is set to
    pub slider_value: SliderValue,
}

#[derive(Component)]
struct SliderProgressEntites {
    empty_bar_entity: Entity,
    bar_entity: Entity,
    square_entity: Entity,
}

fn slider_percent(slider: &Slider, value: &SliderValue) -> f32 {
    if slider.max == slider.min {
        1.0
    } else {
        (value.0 as f32 - slider.min as f32) / ((slider.max) - slider.min) as f32
    }
}

const BASE_SQUARE_SIZE: f32 = 10.0;

const X_MARGIN: f32 = BASE_SQUARE_SIZE;
const Y_MARGIN: f32 = BASE_SQUARE_SIZE / 2.0;

fn on_add_slider(mut commands: Commands, mut q_added_slider: Query<(Entity, &mut Style, &Slider, &SliderValue), Added<Slider>>) {
    for (ent, mut style, slider, slider_value) in q_added_slider.iter_mut() {
        style.height = Val::Px(slider.height + BASE_SQUARE_SIZE);

        let mut bar_entity = None;
        let mut square_entity = None;
        let mut empty_bar_entity = None;

        info!("Slider init value: {slider_value:?}");

        commands.entity(ent).insert(Interaction::default()).with_children(|p| {
            empty_bar_entity = Some(
                p.spawn(NodeBundle {
                    background_color: slider.background_color.into(),
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(slider.height),
                        margin: UiRect {
                            left: Val::Px(X_MARGIN),
                            right: Val::Px(X_MARGIN),
                            top: Val::Px(Y_MARGIN),
                            bottom: Val::Px(Y_MARGIN),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|p| {
                    let percent_selected = slider_percent(slider, slider_value);

                    let square_size = slider.height + BASE_SQUARE_SIZE;

                    bar_entity = Some(
                        p.spawn((
                            Name::new("Slider bar"),
                            NodeBundle {
                                background_color: slider.foreground_color.into(),
                                style: Style {
                                    width: Val::Percent(percent_selected),
                                    height: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id(),
                    );

                    square_entity = Some(
                        p.spawn((
                            Name::new("Slider square"),
                            NodeBundle {
                                background_color: slider.square_color.into(),
                                style: Style {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(square_size),
                                    height: Val::Px(square_size),
                                    left: Val::Px(-BASE_SQUARE_SIZE),
                                    top: Val::Px(-BASE_SQUARE_SIZE / 2.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id(),
                    );
                })
                .id(),
            );
        });

        commands.entity(ent).insert(SliderProgressEntites {
            bar_entity: bar_entity.expect("Set above"),
            square_entity: square_entity.expect("Set above"),
            empty_bar_entity: empty_bar_entity.expect("Set above"),
        });
    }
}

fn on_interact_slider(
    ui_scale: Res<UiScale>,
    mut q_sliders: Query<
        (
            Ref<Interaction>,
            &Slider,
            &mut SliderValue,
            &Node,
            &GlobalTransform,
            &SliderProgressEntites,
        ),
        Without<Disabled>,
    >,
    mut q_bg_color: Query<&mut BackgroundColor>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    for (interaction, slider, mut slider_value, node, g_trans, progress_entities) in q_sliders.iter_mut() {
        if *interaction == Interaction::Pressed {
            let Ok(window) = q_windows.get_single() else {
                continue;
            };

            let Some(cursor_pos) = window.cursor_position() else {
                continue;
            };

            let mut slider_bounds = node.physical_rect(g_trans, 1.0, ui_scale.0);
            slider_bounds.min.x += X_MARGIN;
            slider_bounds.max.x -= X_MARGIN;

            slider_value.0 = if cursor_pos.x <= slider_bounds.min.x {
                slider.min
            } else if cursor_pos.x >= slider_bounds.max.x {
                slider.max
            } else {
                (((cursor_pos.x - slider_bounds.min.x) / (slider_bounds.max.x - slider_bounds.min.x))
                    * ((slider.max) as f32 - slider.min as f32)
                    + slider.min as f32)
                    .round() as i64
            };
        }

        if interaction.is_changed() {
            if let Some(slider_styles) = &slider.slider_styles {
                if let Ok(mut bg_color) = q_bg_color.get_mut(progress_entities.empty_bar_entity) {
                    bg_color.0 = match *interaction {
                        Interaction::None => slider.background_color,
                        Interaction::Hovered => slider_styles.hover_background_color,
                        Interaction::Pressed => slider_styles.press_background_color,
                    };
                }
                if let Ok(mut bg_color) = q_bg_color.get_mut(progress_entities.bar_entity) {
                    bg_color.0 = match *interaction {
                        Interaction::None => slider.foreground_color,
                        Interaction::Hovered => slider_styles.hover_foreground_color,
                        Interaction::Pressed => slider_styles.press_foreground_color,
                    };
                }
            }
        }
    }
}

fn on_change_value(
    mut q_style: Query<&mut Style>,
    ui_scale: Res<UiScale>,
    // Changed<SliderValue> fails here when SliderValue isn't the default value when the slider is just created.
    q_slider_value: Query<(&SliderProgressEntites, &SliderValue, &Slider, &Node, &GlobalTransform)>,
) {
    for (slider_progress_entity, slider_value, slider, node, g_trans) in q_slider_value.iter() {
        let Ok(mut style) = q_style.get_mut(slider_progress_entity.bar_entity) else {
            continue;
        };

        style.width = Val::Percent(slider_percent(slider, slider_value) * 100.0);

        let Ok(mut style) = q_style.get_mut(slider_progress_entity.square_entity) else {
            continue;
        };

        let slider_bounds = node.physical_rect(g_trans, 1.0, ui_scale.0);
        let slider_actual_width = slider_bounds.size().x - X_MARGIN * 2.0;

        style.left = Val::Px(slider_actual_width * slider_percent(slider, slider_value) - BASE_SQUARE_SIZE);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`Slider`]` component uses. Make sure you add any [`Slider`] components before this set!
pub enum SliderUiSystemSet {
    /// Make sure you add any [`Slider`] components before this set!
    ///
    /// Sets up any [`Slider`] components added.
    AddSliderBundle,
    /// Updates the slider to contain any values that have been set via the "react" system
    HandleReactValues,
    /// Sends user events from the various [`Slider`] components.
    SliderInteraction,
    /// Sends user events from the various [`Slider`] components.
    UpdateSliderDisplay,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            SliderUiSystemSet::AddSliderBundle,
            SliderUiSystemSet::HandleReactValues,
            SliderUiSystemSet::SliderInteraction,
            SliderUiSystemSet::UpdateSliderDisplay,
        )
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            on_add_slider.in_set(SliderUiSystemSet::AddSliderBundle),
            on_interact_slider.in_set(SliderUiSystemSet::SliderInteraction),
            on_change_value.in_set(SliderUiSystemSet::UpdateSliderDisplay),
        ),
    )
    .register_type::<SliderValue>()
    .register_type::<Slider>();
}
