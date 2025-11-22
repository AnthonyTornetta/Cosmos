//! A UI component that is used to scroll through a larger UI element.

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::{PrimaryWindow, Window},
};

use crate::ui::UiSystemSet;

use super::Disabled;

#[derive(Component, Default, Debug, Reflect)]
#[require(Node)]
/// Put content you want to scroll through as a child of this
pub struct ScrollBox {
    /// The amount that is scrolled by (in pixels)
    pub scroll_amount: Val,
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
            scrollbar_background_color: Srgba::hex("555555").unwrap().into(),
            // hover_scrollbar_background_color: Color::GRAY,
            // press_scrollbar_background_color: Color::AQUAMARINE,
            scrollbar_color: Srgba::hex("999999").unwrap().into(),
            // hover_scrollbar_color: Color::GRAY,
            // press_scrollbar_color: Color::AQUAMARINE,
        }
    }
}

#[derive(Component)]
struct ContentsContainer(Entity);

#[derive(Component)]
struct ScrollbarContainerEntity(Entity);

#[derive(Component)]
struct ScrollbarEntity(Entity);

fn on_add_scrollbar(mut commands: Commands, mut q_added_button: Query<(Entity, &ScrollBox, &mut Node, &Children), Added<ScrollBox>>) {
    for (ent, scrollbox, mut style, children) in q_added_button.iter_mut() {
        style.overflow = Overflow::clip_y();

        let container_entity = commands
            .spawn((
                Name::new("Scrollbar Content Container"),
                Node {
                    // Take the size of the parent node.
                    flex_grow: 1.0,
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    padding: UiRect::right(Val::Px(20.0)),
                    min_height: Val::Percent(100.0),
                    ..Default::default()
                },
            ))
            .id();

        let scroll_bar = commands
            .spawn((
                Name::new("Scrollbar Container"),
                Interaction::None,
                Node {
                    // Take the size of the parent node.
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0), // aligns it to right
                    width: Val::Px(15.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                BackgroundColor(scrollbox.styles.scrollbar_background_color),
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Scrollbar"),
                    ScrollbarContainerEntity(ent),
                    Interaction::None,
                    Node {
                        // Take the size of the parent node.
                        position_type: PositionType::Relative,
                        top: Val::Percent(0.0),
                        width: Val::Px(15.0),
                        height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,

                        ..Default::default()
                    },
                    BackgroundColor(scrollbox.styles.scrollbar_color),
                ));
            })
            .id();

        for child in children.iter() {
            commands.entity(child).insert(ChildOf(container_entity));
        }

        commands.entity(ent).add_child(container_entity).add_child(scroll_bar).insert((
            Interaction::None,
            ContentsContainer(container_entity),
            ScrollbarEntity(scroll_bar),
        ));
    }
}

fn compute_scroll_px(scroll_amount: Val, items_height: f32, container_height: f32) -> f32 {
    let max_scroll = (items_height - container_height).max(0.0);
    match scroll_amount {
        Val::Px(px) => px,
        Val::Percent(perc) => max_scroll * perc / 100.0,
        Val::Auto => 0.0,
        _ => panic!("Not supported by scroll bar (yet)"),
    }
}

fn on_interact_slider(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    mut q_scroll_containers: Query<
        (
            &mut ScrollBox,
            &Interaction,
            &ComputedNode,
            &ContentsContainer,
            &ScrollbarEntity,
            &GlobalTransform,
        ),
        Without<Disabled>,
    >,
    q_container: Query<&ComputedNode>,
    input: Res<ButtonInput<KeyCode>>,
    mouse_btns: Res<ButtonInput<MouseButton>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_interaction: Query<&Interaction>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scrollbar, interaction, node, contents_container, _, _) in &mut q_scroll_containers {
            if *interaction == Interaction::None {
                continue;
            }

            let Ok(contents_node) = q_container.get(contents_container.0) else {
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

            let as_px = compute_scroll_px(scrollbar.scroll_amount, items_height, container_height);

            let new_amount = (as_px - dy).clamp(0.0, max_scroll);
            scrollbar.scroll_amount = Val::Px(new_amount);
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

            let Ok(contents_node) = q_container.get(contents_container.0) else {
                error!("This should never happen - contents has no style/node.");
                continue;
            };

            let Ok(window) = q_windows.single() else {
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

            let t = g_trans.translation();
            let phys_rect = Rect::from_center_size(Vec2::new(t.x, t.y), node.size());

            let min = phys_rect.min.y + scrollbar_height_px / 2.0;
            let max = phys_rect.max.y - scrollbar_height_px / 2.0;

            let mouse_percent = ((cursor_pos.y - min) / (max - min)).clamp(0.0, 1.0);

            let amount = mouse_percent * max_scroll;
            scrollbar.scroll_amount = Val::Px(amount);
        }
    }
}

fn cap_scroll_to_parent_height(
    mut q_scroll_containers: Query<(&mut ScrollBox, &ComputedNode, &ContentsContainer)>,
    q_container: Query<&ComputedNode>,
) {
    for (mut scrollbar, node, contents_container) in q_scroll_containers.iter_mut() {
        let Ok(contents_node) = q_container.get(contents_container.0) else {
            error!("This should never happen - contents has no style/node.");
            continue;
        };

        let items_height = contents_node.size().y;
        let container_height = node.size().y;

        let as_px = compute_scroll_px(scrollbar.scroll_amount, items_height, container_height);
        let max_scroll = (items_height - container_height).max(0.0);
        if as_px > max_scroll {
            scrollbar.scroll_amount = Val::Px(max_scroll);
        }
    }
}

fn on_change_scrollbar(
    q_scroll_containers: Query<(&ScrollBox, &ComputedNode, &ContentsContainer), Changed<ScrollBox>>,
    mut q_container: Query<(&mut Node, &ComputedNode)>,
) {
    for (scrollbar, node, contents_container) in q_scroll_containers.iter() {
        let Ok((mut style, contents_node)) = q_container.get_mut(contents_container.0) else {
            error!("This should never happen - contents has no style/node.");
            continue;
        };

        let items_height = contents_node.size().y;
        let container_height = node.size().y;

        let as_px = compute_scroll_px(scrollbar.scroll_amount, items_height, container_height);

        style.top = Val::Px(-as_px);
    }
}

fn handle_scrollbar(
    mut query_list: Query<(&ScrollbarContainerEntity, &mut Node)>,
    q_scroll_container: Query<(&ComputedNode, &ScrollBox, &ContentsContainer)>,
    q_node: Query<&ComputedNode>,
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
            let as_px = compute_scroll_px(scrollbar.scroll_amount, items_height, container_height);

            style.top = Val::Px(as_px / (items_height - container_height) * (container_height - scrollbar_height_px));
        } else {
            style.top = Val::Px(0.0);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`ScrollBox`]` component uses. Make sure you add any [`ScrollBox`] components before this set!
pub enum ScrollBoxUiSystemSet {
    /// Make sure you add any [`ScrollBox`] components before this set!
    ///
    /// Sets up any [`ScrollBox`] components added.
    AddScrollBoxBundle,
    /// Sends user events from the various [`ScrollBox`] components.
    ScrollBoxInteraction,
    /// Sends user events from the various [`ScrollBox`] components.
    UpdateScrollBoxDisplay,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ScrollBoxUiSystemSet::AddScrollBoxBundle,
            ScrollBoxUiSystemSet::ScrollBoxInteraction,
            ScrollBoxUiSystemSet::UpdateScrollBoxDisplay,
        )
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            on_add_scrollbar.in_set(ScrollBoxUiSystemSet::AddScrollBoxBundle),
            on_interact_slider.in_set(ScrollBoxUiSystemSet::ScrollBoxInteraction),
            (handle_scrollbar, cap_scroll_to_parent_height, on_change_scrollbar)
                .chain()
                .in_set(ScrollBoxUiSystemSet::UpdateScrollBoxDisplay),
        ),
    )
    .register_type::<ScrollBox>();
}
