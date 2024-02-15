//! A UI component that is used to select a number between a range of values using a slider.
//!
//! Similar to the HTML `input type="range"`.use std::ops::Range;

use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{Added, With},
        schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::{BuildChildren, Children},
    input::{
        keyboard::KeyCode,
        mouse::{MouseButton, MouseScrollUnit, MouseWheel},
        Input,
    },
    log::error,
    reflect::Reflect,
    render::color::Color,
    transform::components::GlobalTransform,
    ui::{node_bundles::NodeBundle, FlexDirection, Interaction, Node, Overflow, PositionType, Style, UiRect, UiScale, Val},
    window::{PrimaryWindow, Window},
};

use crate::ui::UiSystemSet;

#[derive(Component, Default, Debug)]
/// Put content you want to scroll through as a child of this
pub struct ScrollBox {
    /// The amount that is scrolled by
    pub scroll_amount: f32,
    /// The styles of this scroll box
    pub styles: ScrollerStyles,
}

#[derive(Debug, Reflect)]
/// Styles to further customize the scrollbox
pub struct ScrollerStyles {
    /// The color of the scrollbar
    pub scrollbar_color: Color,
    // /// The color of the scrollbar when hovered
    // pub hover_scrollbar_color: Color,
    // /// The color of the scrollbar when scrolled
    // pub press_scrollbar_color: Color,
    /// The color of the scrollbar
    pub scrollbar_background_color: Color,
    // /// The color of the scrollbar when hovered
    // pub hover_scrollbar_background_color: Color,
    // /// The color of the scrollbar when scrolled
    // pub press_scrollbar_background_color: Color,
}

impl Default for ScrollerStyles {
    fn default() -> Self {
        Self {
            scrollbar_background_color: Color::hex("555555").unwrap(),
            // hover_scrollbar_background_color: Color::GRAY,
            // press_scrollbar_background_color: Color::AQUAMARINE,
            scrollbar_color: Color::hex("999999").unwrap(),
            // hover_scrollbar_color: Color::GRAY,
            // press_scrollbar_color: Color::AQUAMARINE,
        }
    }
}

#[derive(Debug, Bundle, Default)]
/// Put stuff you want to scroll through as a child of this bundle
pub struct ScrollBundle {
    /// The node bundle that will be used with the Scrollbar
    pub node_bundle: NodeBundle,
    /// The slider component
    pub slider: ScrollBox,
}

#[derive(Component)]
struct ContentsContainer(Entity);

#[derive(Component)]
struct ScrollbarContainerEntity(Entity);

#[derive(Component)]
struct ScrollbarEntity(Entity);

fn on_add_scrollbar(mut commands: Commands, mut q_added_button: Query<(Entity, &ScrollBox, &mut Style, &Children), Added<ScrollBox>>) {
    for (ent, scrollbox, mut style, children) in q_added_button.iter_mut() {
        style.overflow = Overflow::clip_y();

        let container_entity = commands
            .spawn((
                Name::new("Scrollbar Content Container"),
                NodeBundle {
                    style: Style {
                        // Take the size of the parent node.
                        flex_grow: 1.0,
                        position_type: PositionType::Absolute,
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        padding: UiRect {
                            right: Val::Px(15.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();

        let scroll_bar = commands
            .spawn((
                Name::new("Scrollbar Container"),
                Interaction::None,
                NodeBundle {
                    style: Style {
                        // Take the size of the parent node.
                        position_type: PositionType::Relative,
                        margin: UiRect {
                            left: Val::Auto, // aligns it to right
                            ..Default::default()
                        },
                        width: Val::Px(15.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    background_color: scrollbox.styles.scrollbar_background_color.into(),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Scrollbar"),
                    ScrollbarContainerEntity(ent),
                    Interaction::None,
                    NodeBundle {
                        style: Style {
                            // Take the size of the parent node.
                            position_type: PositionType::Relative,
                            top: Val::Percent(0.0),
                            width: Val::Px(15.0),
                            height: Val::Px(0.0),
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        background_color: scrollbox.styles.scrollbar_color.into(),
                        ..Default::default()
                    },
                ));
            })
            .id();

        for &child in children.iter() {
            commands.entity(child).set_parent(container_entity);
        }

        commands.entity(ent).add_child(container_entity).add_child(scroll_bar).insert((
            Interaction::None,
            ContentsContainer(container_entity),
            ScrollbarEntity(scroll_bar),
        ));
    }
}

fn on_interact_slider(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut q_scroll_containers: Query<(
        &mut ScrollBox,
        &Interaction,
        &Node,
        &ContentsContainer,
        &ScrollbarEntity,
        &GlobalTransform,
    )>,
    mut q_container: Query<(&mut Style, &Node)>,
    input: Res<Input<KeyCode>>,
    scale: Res<UiScale>,
    mouse_btns: Res<Input<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_interaction: Query<&Interaction>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scrollbar, interaction, node, contents_container, _, _) in &mut q_scroll_containers {
            if *interaction == Interaction::None {
                continue;
            }

            let Ok((mut style, contents_node)) = q_container.get_mut(contents_container.0) else {
                error!("This should never happen - contents has no style/node.");
                continue;
            };

            let items_height = contents_node.size().y;
            let container_height = node.size().y;

            let max_scroll = (items_height - container_height).max(0.0);

            let scroll_speed_multiplier = if input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight) {
                5.0
            } else {
                1.0
            };

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.0,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            } * scroll_speed_multiplier;

            scrollbar.scroll_amount -= dy;
            scrollbar.scroll_amount = scrollbar.scroll_amount.clamp(0.0, max_scroll);
            style.top = Val::Px(-scrollbar.scroll_amount);
        }
    }

    if mouse_btns.pressed(MouseButton::Left) {
        for (mut scrollbar, _, node, contents_container, scrollbar_entity, g_trans) in q_scroll_containers.iter_mut() {
            let Ok(interaction) = q_interaction.get(scrollbar_entity.0) else {
                continue;
            };

            if *interaction != Interaction::Pressed {
                continue;
            }

            let Ok((mut style, contents_node)) = q_container.get_mut(contents_container.0) else {
                error!("This should never happen - contents has no style/node.");
                continue;
            };

            let Ok(window) = q_windows.get_single() else {
                continue;
            };

            let Some(cursor_pos) = window.cursor_position() else {
                continue;
            };

            let items_height = contents_node.size().y;
            let container_height = node.size().y;
            let scroll_height = (container_height / items_height).min(1.0);
            let scrollbar_height_px = scroll_height * container_height;

            let max_scroll = (items_height - container_height).max(0.0);

            let phys_rect = node.physical_rect(g_trans, 1.0, scale.0);

            let min = phys_rect.min.y + scrollbar_height_px / 2.0;
            let max = phys_rect.max.y - scrollbar_height_px / 2.0;

            let mouse_percent = ((cursor_pos.y - min) / (max - min)).max(0.0).min(1.0);

            scrollbar.scroll_amount = mouse_percent * max_scroll;
            style.top = Val::Px(-scrollbar.scroll_amount);
        }
    }
}

fn handle_scrollbar(
    mut query_list: Query<(&ScrollbarContainerEntity, &mut Style)>,
    q_scroll_container: Query<(&Node, &ScrollBox, &ContentsContainer)>,
    q_node: Query<&Node>,
) {
    for (scrollbar_entity, mut style) in query_list.iter_mut() {
        let Ok((node, scrollbar, contents_container)) = q_scroll_container.get(scrollbar_entity.0) else {
            continue;
        };

        let Ok(contents_node) = q_node.get(contents_container.0) else {
            continue;
        };

        let items_height = contents_node.size().y;
        let container_height = node.size().y;

        let scroll_height = (container_height / items_height).min(1.0);

        style.height = Val::Percent(scroll_height * 100.0);

        let scrollbar_height_px = scroll_height * container_height;

        if scroll_height != 1.0 {
            style.top = Val::Px(scrollbar.scroll_amount / (items_height - container_height) * (container_height - scrollbar_height_px));
        } else {
            style.top = Val::Px(0.0);
        }
    }
}

// https://github.com/bevyengine/bevy/pull/9822
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`Button`]` component uses. Make sure you add any [`Button`] components before this set!
pub enum SliderUiSystemSet {
    /// apply_deferred
    ApplyDeferredA,
    /// Make sure you add any [`Button`] components before this set!
    ///
    /// Sets up any [`Button`] components added.
    AddSliderBundle,
    /// apply_deferred
    ApplyDeferredB,
    /// Sends user events from the various [`Button`] components.
    SliderInteraction,
    /// apply_deferred
    ApplyDeferredC,
    /// Sends user events from the various [`Button`] components.
    UpdateSliderDisplay,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            SliderUiSystemSet::ApplyDeferredA,
            SliderUiSystemSet::AddSliderBundle,
            SliderUiSystemSet::ApplyDeferredB,
            SliderUiSystemSet::SliderInteraction,
            SliderUiSystemSet::ApplyDeferredC,
            SliderUiSystemSet::UpdateSliderDisplay,
        )
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(SliderUiSystemSet::ApplyDeferredA),
            apply_deferred.in_set(SliderUiSystemSet::ApplyDeferredB),
            apply_deferred.in_set(SliderUiSystemSet::ApplyDeferredC),
        ),
    )
    .add_systems(
        Update,
        (
            on_add_scrollbar.in_set(SliderUiSystemSet::AddSliderBundle),
            on_interact_slider.in_set(SliderUiSystemSet::SliderInteraction),
            handle_scrollbar.in_set(SliderUiSystemSet::UpdateSliderDisplay),
        ),
    );
}
