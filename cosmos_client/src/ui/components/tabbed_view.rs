//! A wrapper around ui components that will make them movable and have a title bar with a close button.

use bevy::{
    app::{App, Update},
    color::{Color, Srgba, palettes::css},
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
    log::error,
    prelude::{Changed, ChildBuild, Parent, Without},
    reflect::Reflect,
    text::TextFont,
    ui::{AlignItems, BackgroundColor, Display, FlexDirection, JustifyContent, Node, Val},
    utils::default,
};

use crate::ui::{UiSystemSet, font::DefaultFont};

use super::button::{ButtonEvent, CosmosButton, register_button};

#[derive(Debug, Component, Default)]
/// The tab selected to be viewed - Should be put on the [`TabbedView`] entity.
pub enum SelectedTab {
    #[default]
    /// The first child (also the default displayed)
    Default,
    /// A specific tab (the String is the [`Tab`]'s header)
    Tab(String),
}

#[derive(Debug, Component)]
#[require(Node, SelectedTab)]
/// A wrapper around ui components that will make them movable and have a title bar with a close button.
pub struct TabbedView {
    /// The background color of the view
    pub view_background: BackgroundColor,
    /// The background color of the tabs
    pub tabs_background: BackgroundColor,
    /// The node the body of the tab views will use
    pub body_styles: Node,
}

impl Default for TabbedView {
    fn default() -> Self {
        Self {
            view_background: BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
            tabs_background: BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
            body_styles: Default::default(),
        }
    }
}

impl TabbedView {}

#[derive(Event, Debug)]
struct ClickTabEvent(Entity);

impl ButtonEvent for ClickTabEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

#[derive(Component, Reflect, Debug, Clone)]
/// This child of a [`TabbedView`] will only be shown if the tab with this name is selected
pub struct Tab {
    header: String,
}

impl Tab {
    /// This child of a [`TabbedView`] will only be shown if the tab with this name is selected
    pub fn new(header: impl Into<String>) -> Self {
        Self { header: header.into() }
    }
}

#[derive(Component)]
struct TabbedViewBody;
#[derive(Component)]
struct TabBar;

fn add_tab_view(
    mut commands: Commands,
    mut q_added_tabbed_view: Query<(Entity, &TabbedView, &SelectedTab, &Children, &mut Node), Added<TabbedView>>,
    font: Res<DefaultFont>,
    q_tab: Query<&Tab>,
    mut q_node: Query<&mut Node, (Without<TabbedView>, With<Tab>)>,
) {
    for (ent, tabbed_view, selected_tab, children, mut style) in &mut q_added_tabbed_view {
        style.flex_direction = FlexDirection::Column;

        let font = &font.0;

        let mut window_body = None;

        let tabs = children
            .iter()
            .map(|x| q_tab.get(*x).map(|y| (*x, y)))
            .flatten()
            .collect::<Vec<_>>();

        commands.entity(ent).with_children(|parent| {
            parent
                .spawn((
                    Name::new("Tabs Bar"),
                    TabBar,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        height: Val::Px(60.0),

                        ..default()
                    },
                    tabbed_view.tabs_background,
                ))
                .with_children(|p| {
                    for (idx, &(_, tab)) in tabs.iter().enumerate() {
                        let mut ecmds = p.spawn((
                            Name::new(format!("Tab: {}", tab.header)),
                            tab.clone(),
                            CosmosButton::<ClickTabEvent> {
                                text: Some((
                                    tab.header.clone(),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..Default::default()
                                    },
                                    Default::default(),
                                )),
                                ..Default::default()
                            },
                            Node {
                                flex_grow: 1.0,
                                height: Val::Percent(100.0),
                                ..Default::default()
                            },
                        ));

                        let selected = match selected_tab {
                            SelectedTab::Default => idx == 0,
                            SelectedTab::Tab(t) => &tab.header == t,
                        };
                        if selected {
                            ecmds.insert(BackgroundColor(css::GREY.into()));
                        }
                    }
                });

            window_body = Some(
                parent
                    .spawn((
                        Name::new("Tab Body"),
                        tabbed_view.view_background,
                        TabbedViewBody,
                        Node {
                            flex_grow: 1.0,
                            ..tabbed_view.body_styles.clone()
                        },
                    ))
                    .id(),
            );
        });

        for (idx, &(tab_ent, tab)) in tabs.iter().enumerate() {
            let window_body = window_body.expect("Set above");
            if let Ok(mut node) = q_node.get_mut(tab_ent) {
                match selected_tab {
                    SelectedTab::Default => {
                        if idx != 0 {
                            node.display = Display::None;
                        }
                    }
                    SelectedTab::Tab(t) => {
                        if &tab.header != t {
                            node.display = Display::None;
                        }
                    }
                }
                commands.entity(tab_ent).set_parent(window_body);
            }
        }
    }
}

fn on_change_selected(
    q_changed_selected: Query<(&SelectedTab, &Children), (With<TabbedView>, Changed<SelectedTab>)>,
    mut q_tab: Query<(&Tab, &mut Node)>,
    q_tab_bar: Query<&Children, With<TabBar>>,
    q_tabbed_body: Query<&Children, With<TabbedViewBody>>,
    mut q_bg_color: Query<&mut BackgroundColor>,
) {
    for (selected_tab, children) in q_changed_selected.iter() {
        let Some(tabbed_bar_children) = children.iter().flat_map(|x| q_tab_bar.get(*x)).next() else {
            continue;
        };

        let Some(children) = children.iter().flat_map(|x| q_tabbed_body.get(*x)).next() else {
            continue;
        };

        let mut first = true;
        for (&child, &tab_ent) in children.iter().zip(tabbed_bar_children.iter()) {
            let Ok((tab, mut node)) = q_tab.get_mut(child) else {
                continue;
            };

            let selected = match selected_tab {
                SelectedTab::Default => first,
                SelectedTab::Tab(t) => t == &tab.header,
            };

            if selected {
                node.display = Display::Flex;
                if let Ok(mut bg) = q_bg_color.get_mut(tab_ent) {
                    bg.0 = css::GREY.into();
                }
            } else {
                node.display = Display::None;
                if let Ok(mut bg) = q_bg_color.get_mut(tab_ent) {
                    bg.0 = Color::NONE;
                }
            }

            first = false;
        }
    }
}

fn on_click_tab(
    q_parent: Query<&Parent>,
    q_tab: Query<&Tab>,
    mut q_selected_tab: Query<&mut SelectedTab>,
    mut evr_tab_clicked: EventReader<ClickTabEvent>,
) {
    for ev in evr_tab_clicked.read() {
        let Ok(tab) = q_tab.get(ev.0) else {
            error!("No tab component on tab!");
            continue;
        };
        let Ok(p) = q_parent.get(ev.0).and_then(|e| q_parent.get(e.get())) else {
            error!("Invalid UI heirarchy");
            continue;
        };

        let Ok(mut selected) = q_selected_tab.get_mut(p.get()) else {
            error!("Unable to get selected tab component");
            continue;
        };

        *selected = SelectedTab::Tab(tab.header.clone());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// UI Window system set
pub enum UiTabViewSystemSet {
    /// Creates the window
    CreateTabView,
    /// Events such as closing and moving the window are performed
    SendTabViewEvents,
}

pub(super) fn register(app: &mut App) {
    register_button::<ClickTabEvent>(app);

    app.configure_sets(
        Update,
        (UiTabViewSystemSet::CreateTabView, UiTabViewSystemSet::SendTabViewEvents).in_set(UiSystemSet::DoUi),
    );

    app.add_systems(
        Update,
        (
            add_tab_view.in_set(UiTabViewSystemSet::CreateTabView),
            (on_click_tab, on_change_selected)
                .chain()
                .in_set(UiTabViewSystemSet::SendTabViewEvents),
        ),
    )
    .register_type::<Tab>();
}
