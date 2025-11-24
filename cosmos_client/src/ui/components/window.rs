//! A wrapper around ui components that will make them movable and have a title bar with a close button.

use bevy::{color::palettes::css, prelude::*, window::PrimaryWindow};
use cosmos_core::{ecs::NeedsDespawned, state::GameState};

use crate::{
    asset::asset_loader::load_assets,
    ui::{UiSystemSet, font::DefaultFont},
    window::setup::DeltaCursorPosition,
};

use super::{
    button::{ButtonEvent, CosmosButton},
    show_cursor::{ShowCursor, any_open_menus},
};

#[derive(Debug, Component)]
#[require(Node, ShowCursor)]
/// A wrapper around ui components that will make them movable and have a title bar with a close button.
pub struct GuiWindow {
    /// The title that should be displayed
    pub title: String,
    /// Styles that effect the wrapper around the children of the window node
    pub body_styles: Node,
    /// The window's bacground color
    pub window_background: BackgroundColor,
}

impl Default for GuiWindow {
    fn default() -> Self {
        Self {
            title: Default::default(),
            body_styles: Default::default(),
            window_background: BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
        }
    }
}

impl GuiWindow {
    /// The height of a window's title bar
    pub const TITLE_BAR_HEIGHT_PX: f32 = 60.0;
}

#[derive(Component)]
struct TitleBar {
    window_entity: Entity,
}

#[derive(Resource, Debug)]
/// The assets used by the [`GuiWindow`]
pub struct WindowAssets {
    /// The image used for the titlebar
    pub title_bar_image: Handle<Image>,
    /// The image used for the close button
    pub close_btn_image: Handle<Image>,
}

fn add_window(
    mut commands: Commands,
    mut q_added_window: Query<(Entity, &GuiWindow, Option<&Children>, &mut Node), Added<GuiWindow>>,
    font: Res<DefaultFont>,
    q_title_bar: Query<(), With<GuiWindowTitleBar>>,
    window_assets: Res<WindowAssets>,
) {
    for (ent, window, children, mut style) in &mut q_added_window {
        style.flex_direction = FlexDirection::Column;

        let font = &font.0;

        let mut window_body = None;

        let close_button = CloseButton(ent);

        let titlebar_children = children
            .map(|x| x.iter().filter(|x| q_title_bar.contains(*x)).collect::<Vec<_>>())
            .unwrap_or_default();

        style.border = UiRect::all(Val::Px(2.0));

        commands
            .entity(ent)
            .insert((BorderColor::all(Srgba::hex("#111").unwrap()), GlobalZIndex(5)))
            .with_children(|parent| {
                // Title bar

                let mut title_bar = parent.spawn((
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
                    ImageNode::new(window_assets.title_bar_image.clone()),
                ));

                title_bar.with_children(|parent| {
                    parent.spawn((
                        Name::new("Title Text"),
                        Text::new(&window.title),
                        TextFont {
                            font_size: 24.0,
                            font: font.clone(),
                            ..Default::default()
                        },
                        TextLayout {
                            justify: Justify::Center,
                            ..Default::default()
                        },
                    ));
                });

                for child in titlebar_children {
                    title_bar.add_child(child);
                }

                title_bar.with_children(|parent| {
                    parent
                        .spawn((
                            Name::new("Window Close Button"),
                            close_button,
                            BackgroundColor(css::WHITE.into()),
                            Node {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                ..Default::default()
                            },
                            CosmosButton {
                                image: Some(ImageNode::new(window_assets.close_btn_image.clone())),
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
                        ))
                        .observe(close_event_listener);
                });

                window_body = Some(
                    parent
                        .spawn((
                            Name::new("Window Body"),
                            window.window_background,
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
                if !q_title_bar.contains(child) {
                    commands.entity(child).insert(ChildOf(window_body));
                }
            }
        }
    }
}

#[derive(Component, Debug)]
/// If something is the child of a [`GuiWindow`] with this component, this will be moved to be a child of
/// the title bar created by the [`GuiWindow`].
pub struct GuiWindowTitleBar;

fn move_window(
    q_window: Query<&Window, With<PrimaryWindow>>,
    cursor_delta_position: Res<DeltaCursorPosition>,
    mut q_style: Query<(&ComputedNode, &UiGlobalTransform, &mut Node)>,
    q_title_bar: Query<(&Interaction, &TitleBar)>,
) {
    for (interaction, title_bar) in &q_title_bar {
        if *interaction == Interaction::Pressed {
            let Ok(window) = q_window.single() else {
                return;
            };

            let Ok((node, g_trans, mut style)) = q_style.get_mut(title_bar.window_entity) else {
                continue;
            };

            let t = g_trans.translation;
            let bounds = Rect::from_center_size(t, node.size());
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

#[derive(Component, Debug)]
struct CloseButton(Entity);

fn close_event_listener(ev: On<ButtonEvent>, mut commands: Commands, q_close_button: Query<&CloseButton>) {
    let Ok(close_btn) = q_close_button.get(ev.0) else {
        return;
    };

    commands.entity(close_btn.0).insert(NeedsDespawned);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// UI Window system set
pub enum UiWindowSystemSet {
    /// Creates the window
    CreateWindow,
    /// Messages such as closing and moving the window are performed
    SendWindowMessages,
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

    app.configure_sets(
        Update,
        (UiWindowSystemSet::CreateWindow, UiWindowSystemSet::SendWindowMessages).in_set(UiSystemSet::DoUi),
    );

    app.add_systems(
        Update,
        (
            add_window
                .in_set(UiWindowSystemSet::CreateWindow)
                .run_if(resource_exists::<WindowAssets>),
            move_window.run_if(any_open_menus).in_set(UiWindowSystemSet::SendWindowMessages),
        ),
    );
}
