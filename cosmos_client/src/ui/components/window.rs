//! A wrapper around ui components that will make them movable and have a title bar with a close button.

use bevy::{
    app::{App, Update},
    asset::Handle,
    color::palettes::css,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, With},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::{BuildChildren, Children},
    image::Image,
    math::{Rect, Vec2},
    prelude::{resource_exists, ChildBuild, ImageNode, Resource, Text},
    text::{JustifyText, TextFont, TextLayout},
    transform::components::GlobalTransform,
    ui::{AlignItems, BackgroundColor, ComputedNode, Display, FlexDirection, Interaction, JustifyContent, Node, PositionType, UiRect, Val},
    utils::default,
    window::{PrimaryWindow, Window},
};
use cosmos_core::{ecs::NeedsDespawned, state::GameState};

use crate::{
    asset::asset_loader::load_assets,
    ui::{font::DefaultFont, UiSystemSet},
    window::setup::DeltaCursorPosition,
};

use super::{
    button::{register_button, ButtonEvent, CosmosButton},
    show_cursor::{any_open_menus, ShowCursor},
};

#[derive(Debug, Component, Default)]
#[require(Node, ShowCursor)]
/// A wrapper around ui components that will make them movable and have a title bar with a close button.
pub struct GuiWindow {
    /// The title that should be displayed
    pub title: String,
    /// Styles that effect the wrapper around the children of the window node
    pub body_styles: Node,
}

impl GuiWindow {
    /// The height of a window's title bar
    pub const TITLE_BAR_HEIGHT_PX: f32 = 60.0;
}

#[derive(Event, Debug)]
struct CloseUiEvent(Entity);

#[derive(Component, Debug)]
struct CloseButton {
    window_entity: Entity,
}

impl ButtonEvent for CloseUiEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

#[derive(Component)]
struct TitleBar {
    window_entity: Entity,
}

#[derive(Resource, Debug)]
struct WindowAssets {
    title_bar_image: Handle<Image>,
    close_btn_image: Handle<Image>,
}

fn add_window(
    mut commands: Commands,
    mut q_added_window: Query<(Entity, &GuiWindow, Option<&Children>, &mut Node), Added<GuiWindow>>,
    font: Res<DefaultFont>,
    window_assets: Res<WindowAssets>,
) {
    for (ent, window, children, mut style) in &mut q_added_window {
        style.flex_direction = FlexDirection::Column;

        let font = &font.0;

        let mut window_body = None;

        let close_button = CloseButton { window_entity: ent };

        commands.entity(ent).with_children(|parent| {
            // Title bar
            parent
                .spawn((
                    Name::new("Title Bar"),
                    TitleBar { window_entity: ent },
                    Interaction::None,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        height: Val::Px(60.0),
                        padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),

                        ..default()
                    },
                    BackgroundColor(css::WHITE.into()),
                    ImageNode::new(window_assets.title_bar_image.clone_weak()),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Title Text"),
                        Text::new(&window.title),
                        TextFont {
                            font_size: 24.0,
                            font: font.clone(),
                            ..Default::default()
                        },
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                    ));

                    parent.spawn((
                        Name::new("Window Close Button"),
                        close_button,
                        BackgroundColor(css::WHITE.into()),
                        Node {
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            ..Default::default()
                        },
                        CosmosButton::<CloseUiEvent> {
                            image: Some(ImageNode::new(window_assets.close_btn_image.clone_weak())),
                            text: Some((
                                "X".into(),
                                TextFont {
                                    font_size: 24.0,
                                    font: font.clone(),
                                    ..Default::default()
                                },
                                Default::default(),
                            )),
                            ..Default::default()
                        },
                    ));
                });

            window_body = Some(
                parent
                    .spawn((
                        Name::new("Window Body"),
                        Node {
                            flex_grow: 1.0,
                            ..window.body_styles.clone()
                        },
                    ))
                    .id(),
            );
        });

        if let Some(children) = children {
            let window_body = window_body.expect("Set above");
            for &child in children {
                commands.entity(child).set_parent(window_body);
            }
        }
    }
}

fn move_window(
    q_window: Query<&Window, With<PrimaryWindow>>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    mut q_style: Query<(&ComputedNode, &GlobalTransform, &mut Node)>,
    q_title_bar: Query<(&Interaction, &TitleBar)>,
) {
    for (interaction, title_bar) in &q_title_bar {
        if *interaction == Interaction::Pressed {
            let Ok(window) = q_window.get_single() else {
                return;
            };

            let Ok((node, g_trans, mut style)) = q_style.get_mut(title_bar.window_entity) else {
                continue;
            };

            let t = g_trans.translation();
            let bounds = Rect::from_center_size(Vec2::new(t.x, t.y), node.size());
            // let bounds = node.logical_rect(g_trans);

            let left = match style.left {
                Val::Px(px) => px,
                _ => bounds.min.x,
            };

            let top = match style.top {
                Val::Px(px) => px,
                _ => bounds.min.y,
            };

            let (max_x, max_y) = (window.width() - 50.0, window.height() - 50.0);
            let (min_x, min_y) = (50.0 - (bounds.max.x - bounds.min.x), 0.0);

            style.left = Val::Px((left + cursor_delta_position.x).clamp(min_x, max_x));
            style.top = Val::Px((top - cursor_delta_position.y).clamp(min_y, max_y));
            if style.position_type != PositionType::Absolute {
                style.position_type = PositionType::Absolute;
            }
        }
    }
}

fn close_event_listener(mut commands: Commands, q_close_button: Query<&CloseButton>, mut ev_reader: EventReader<CloseUiEvent>) {
    for ev in ev_reader.read() {
        let Ok(close_btn) = q_close_button.get(ev.0) else {
            continue;
        };

        commands.entity(close_btn.window_entity).insert(NeedsDespawned);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// UI Window system set
pub enum UiWindowSystemSet {
    /// Creates the window
    CreateWindow,
    /// Events such as closing and moving the window are performed
    SendWindowEvents,
}

pub(super) fn register(app: &mut App) {
    load_assets::<Image, WindowAssets, 2>(
        app,
        GameState::Loading,
        [
            "cosmos/images/ui/inventory-close-button.png",
            "cosmos/images/ui/inventory-header.png",
        ],
        |mut commands, [close_btn_img, header_img]| {
            commands.insert_resource(WindowAssets {
                title_bar_image: header_img.0,
                close_btn_image: close_btn_img.0,
            })
        },
    );

    register_button::<CloseUiEvent>(app);

    app.configure_sets(
        Update,
        (UiWindowSystemSet::CreateWindow, UiWindowSystemSet::SendWindowEvents).in_set(UiSystemSet::DoUi),
    );

    app.add_systems(
        Update,
        (
            add_window
                .in_set(UiWindowSystemSet::CreateWindow)
                .run_if(resource_exists::<WindowAssets>),
            (move_window.run_if(any_open_menus), close_event_listener).in_set(UiWindowSystemSet::SendWindowEvents),
        ),
    );
}
