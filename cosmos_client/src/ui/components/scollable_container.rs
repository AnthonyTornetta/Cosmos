//! A UI component that is used to scroll through a larger UI element.

use bevy::{
    color::palettes::css,
    input::mouse::{MouseScrollUnit, MouseWheel},
    math::Affine2,
    picking::hover::HoverMap,
    prelude::*,
    render::{Extract, sync_world::TemporaryRenderEntity},
    ui_render::{ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, NodeType, RenderUiSystems, UiCameraMap, stack_z_offsets},
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
            &UiGlobalTransform,
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

            let t = g_trans.translation;
            let phys_rect = Rect::from_center_size(t, node.size());

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

const LINE_HEIGHT: f32 = 21.0;

/// Injects scroll events into the UI hierarchy.
fn send_scroll_events(mut mouse_wheel_reader: MessageReader<MouseWheel>, hover_map: Res<HoverMap>, mut commands: Commands) {
    // info!("{hover_map:?}");
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
        info!("{delta}");

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= LINE_HEIGHT;
        }

        for pointer_map in hover_map.values() {
            info!("{pointer_map:?}");
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}

/// UI scrolling event.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    /// Scroll delta in logical coordinates.
    delta: Vec2,
}

fn on_scroll_handler(mut scroll: On<Scroll>, mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>) {
    info!("{scroll:?} :)");
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    info!("{scroll:?} :D");

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    let delta = &mut scroll.delta;
    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0.0 {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.x > 0.0 {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.0
        };

        if !max {
            scroll_position.x += delta.x;
            // Consume the X portion of the scroll delta.
            delta.x = 0.0;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.y > 0. {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.0
        };

        if !max {
            scroll_position.y += delta.y;
            // Consume the Y portion of the scroll delta.
            delta.y = 0.0;
        }
    }

    // Stop propagating when the delta is fully consumed.
    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

/// Styling for an automatic scrollbar
#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, PartialEq, Clone)]
pub struct ScrollbarStyle {
    /// Color of the scrollbar's thumb
    pub thumb: Color,
    /// Color of the scrollbar's gutter
    pub gutter: Color,
    /// Color of the scrollbar's corner section
    pub corner: Color,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            thumb: Color::WHITE,
            gutter: css::GRAY.into(),
            corner: Color::BLACK,
        }
    }
}

/// Compute the size and position of the horizontal scrollbar's gutter
fn horizontal_scrollbar_gutter(uinode: &ComputedNode) -> Rect {
    let content_inset = uinode.content_inset();
    let min_x = content_inset.left;
    let max_x = uinode.size.x - content_inset.right - uinode.scrollbar_size.x;
    let max_y = uinode.size.y - content_inset.bottom;
    let min_y = max_y - uinode.scrollbar_size.y;
    Rect {
        min: (min_x, min_y).into(),
        max: (max_x, max_y).into(),
    }
}

/// Compute the size and position of the vertical scrollbar's gutter
fn vertical_scrollbar_gutter(uinode: &ComputedNode) -> Rect {
    let content_inset = uinode.content_inset();
    let max_x = uinode.size.x - content_inset.right;
    let min_x = max_x - uinode.scrollbar_size.x;
    let min_y = content_inset.top;
    let max_y = uinode.size.y - content_inset.bottom - uinode.scrollbar_size.y;
    Rect {
        min: (min_x, min_y).into(),
        max: (max_x, max_y).into(),
    }
}

// Compute the size and position of the horizontal scrollbar's thumb
fn horizontal_scrollbar_thumb(uinode: &ComputedNode) -> Rect {
    let gutter = horizontal_scrollbar_gutter(uinode);
    let width = gutter.size().x * gutter.size().x / uinode.content_size.x;
    let min_x = gutter.size().x * uinode.scroll_position.x / uinode.content_size.x;
    let min = (min_x, gutter.min.y).into();
    let max = min + Vec2::new(width, gutter.size().y);
    Rect { min, max }
}

// Compute the size and position of the vertical scrollbar's thumb
fn vertical_scrollbar_thumb(uinode: &ComputedNode) -> Rect {
    let gutter = vertical_scrollbar_gutter(uinode);
    let height = gutter.size().y * gutter.size().y / uinode.content_size.y;
    let min_y = gutter.size().y * uinode.scroll_position.y / uinode.content_size.y;
    let min = (gutter.min.x, min_y).into();
    let max = (gutter.max.x, min_y + height).into();
    Rect { min, max }
}

fn extract_scrollbars(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            Option<&ScrollbarStyle>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    for (entity, uinode, transform, inherited_visibility, clip, camera, colors) in &uinode_query {
        // Skip invisible backgrounds
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        if uinode.scrollbar_size.cmple(Vec2::ZERO).all() {
            continue;
        }

        let colors = colors.copied().unwrap_or_default();

        let top_left = transform.translation - 0.5 * uinode.size;

        let h_bar = horizontal_scrollbar_gutter(uinode);
        let v_bar = vertical_scrollbar_gutter(uinode);

        let corner = Rect::from_corners(Vec2::new(v_bar.min.x, h_bar.min.y), Vec2::new(v_bar.max.x, h_bar.max.y));

        let stack_z_offset = stack_z_offsets::TEXT + 0.01;

        if !corner.is_empty() {
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offset,
                clip: clip.map(|clip| clip.clip),
                image: AssetId::default(),
                extracted_camera_entity,
                transform: Affine2::from_translation(top_left + corner.center()),
                item: ExtractedUiItem::Node {
                    color: colors.corner.into(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: corner.size(),
                    },
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border: BorderRect::ZERO,
                    border_radius: ResolvedBorderRadius::ZERO,
                    node_type: NodeType::Rect,
                },
                main_entity: entity.into(),
            });
        }

        for (gutter, thumb) in [
            (h_bar, horizontal_scrollbar_thumb(uinode)),
            (v_bar, vertical_scrollbar_thumb(uinode)),
        ] {
            if gutter.is_empty() {
                continue;
            }
            let transform = Affine2::from_translation(top_left) * Affine2::from_translation(gutter.center());
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offset,
                clip: clip.map(|clip| clip.clip),
                image: AssetId::default(),
                extracted_camera_entity,
                transform,
                item: ExtractedUiItem::Node {
                    color: colors.gutter.into(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: gutter.size(),
                    },
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border: BorderRect::ZERO,
                    border_radius: ResolvedBorderRadius::ZERO,
                    node_type: NodeType::Rect,
                },
                main_entity: entity.into(),
            });

            let transform = Affine2::from_translation(top_left) * Affine2::from_translation(thumb.center());
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offset,
                clip: clip.map(|clip| clip.clip),
                image: AssetId::default(),
                extracted_camera_entity,
                transform,
                item: ExtractedUiItem::Node {
                    color: colors.thumb.into(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: thumb.size(),
                    },
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border: BorderRect::ZERO,
                    border_radius: ResolvedBorderRadius::ZERO,
                    node_type: NodeType::Rect,
                },
                main_entity: entity.into(),
            });
        }
    }
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
        ExtractSchedule,
        extract_scrollbars
            .after(RenderUiSystems::ExtractText)
            .before(RenderUiSystems::ExtractDebug),
    )
    .add_systems(
        Update,
        (
            (send_scroll_events),
            on_add_scrollbar.in_set(ScrollBoxUiSystemSet::AddScrollBoxBundle),
            on_interact_slider.in_set(ScrollBoxUiSystemSet::ScrollBoxInteraction),
            (handle_scrollbar, cap_scroll_to_parent_height, on_change_scrollbar)
                .chain()
                .in_set(ScrollBoxUiSystemSet::UpdateScrollBoxDisplay),
        ),
    )
    .add_observer(on_scroll_handler)
    .register_type::<ScrollBox>();
}
