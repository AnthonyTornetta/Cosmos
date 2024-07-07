//! A wrapper around ui components that will make them movable and have a title bar with a close button.

use bevy::{
    app::{App, Update},
    asset::AssetServer,
    color::palettes::css,
    core::Name,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, With},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::{BuildChildren, Children},
    text::{JustifyText, Text, TextStyle},
    transform::components::GlobalTransform,
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        AlignItems, Display, FlexDirection, Interaction, JustifyContent, Node, PositionType, Style, UiImage, UiRect, Val,
    },
    utils::default,
    window::{PrimaryWindow, Window},
};
use cosmos_core::ecs::NeedsDespawned;

use crate::{ui::UiSystemSet, window::setup::DeltaCursorPosition};

use super::{
    button::{register_button, Button, ButtonBundle, ButtonEvent},
    show_cursor::ShowCursor,
};

#[derive(Debug, Component, Default)]
/// A wrapper around ui components that will make them movable and have a title bar with a close button.
pub struct GuiWindow {
    /// The title that should be displayed
    pub title: String,
    /// Styles that effect the wrapper around the children of the window node
    pub body_styles: Style,
}

#[derive(Bundle, Debug, Default)]
/// A wrapper around ui components that will make them movable and have a title bar with a close button.
pub struct WindowBundle {
    /// A wrapper around ui components that will make them movable and have a title bar with a close button.
    pub window: GuiWindow,
    /// NodeBundle for further customization. This will be the NodeBundle of the entire window.
    ///
    /// To only style the body of the window, change the body_styles in the GuiWindow object.
    pub node_bundle: NodeBundle,
    /// Makes the cursor show itself
    pub show_cursor: ShowCursor,
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

fn add_window(
    mut commands: Commands,
    mut q_added_window: Query<(Entity, &GuiWindow, &Children, &mut Style), Added<GuiWindow>>,
    asset_server: Res<AssetServer>,
) {
    for (ent, window, children, mut style) in &mut q_added_window {
        style.flex_direction = FlexDirection::Column;

        let font = asset_server.load("fonts/PixeloidSans.ttf");

        let mut window_body = None;

        let close_button = CloseButton { window_entity: ent };

        commands.entity(ent).with_children(|parent| {
            // Title bar
            parent
                .spawn((
                    Name::new("Title Bar"),
                    TitleBar { window_entity: ent },
                    Interaction::None,
                    NodeBundle {
                        style: Style {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            width: Val::Percent(100.0),
                            height: Val::Px(60.0),
                            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),

                            ..default()
                        },
                        background_color: css::WHITE.into(),
                        ..default()
                    },
                    UiImage {
                        texture: asset_server.load("cosmos/images/ui/inventory-header.png"),
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Title Text"),
                        TextBundle {
                            style: Style { ..default() },
                            text: Text::from_section(
                                &window.title,
                                TextStyle {
                                    color: css::WHITE.into(),
                                    font_size: 24.0,
                                    font: font.clone(),
                                },
                            )
                            .with_justify(JustifyText::Center),
                            ..default()
                        },
                    ));

                    parent.spawn((
                        Name::new("Window Close Button"),
                        close_button,
                        ButtonBundle {
                            node_bundle: NodeBundle {
                                background_color: css::WHITE.into(),
                                style: Style {
                                    width: Val::Px(50.0),
                                    height: Val::Px(50.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            button: Button::<CloseUiEvent> {
                                image: Some(UiImage {
                                    texture: asset_server.load("cosmos/images/ui/inventory-close-button.png"),
                                    ..Default::default()
                                }),
                                text: Some((
                                    "X".into(),
                                    TextStyle {
                                        color: css::WHITE.into(),
                                        font_size: 24.0,
                                        font: font.clone(),
                                    },
                                )),
                                ..Default::default()
                            },
                        },
                    ));
                });

            window_body = Some(
                parent
                    .spawn((
                        Name::new("Window Body"),
                        NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                ..window.body_styles.clone()
                            },
                            ..Default::default()
                        },
                    ))
                    .id(),
            );
        });

        let window_body = window_body.expect("Set above");
        for &child in children {
            commands.entity(child).set_parent(window_body);
        }
    }
}

fn move_window(
    q_window: Query<&Window, With<PrimaryWindow>>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    mut q_style: Query<(&Node, &GlobalTransform, &mut Style)>,
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

            let bounds = node.logical_rect(g_trans);

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
enum UiWindowSystemSet {
    CreateWindow,
    SendWindowEvents,
}

pub(super) fn register(app: &mut App) {
    register_button::<CloseUiEvent>(app);

    app.configure_sets(
        Update,
        (UiWindowSystemSet::CreateWindow, UiWindowSystemSet::SendWindowEvents).in_set(UiSystemSet::DoUi),
    );

    app.add_systems(
        Update,
        (
            add_window.in_set(UiWindowSystemSet::CreateWindow),
            (move_window, close_event_listener).in_set(UiWindowSystemSet::SendWindowEvents),
        ),
    );
}
