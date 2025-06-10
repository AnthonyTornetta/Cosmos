//! Renders items as 3d models at based off the RenderItem present in a UI element

use bevy::{prelude::*, window::PrimaryWindow};
use cosmos_core::{
    ecs::NeedsDespawned,
    item::Item,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use photo_booth::RenderedItemAtlas;

use crate::lang::Lang;

use super::{UiSystemSet, font::DefaultFont};

pub mod photo_booth;

#[derive(Debug, Component, Reflect)]
/// Put this onto a UI element to render a 3D item there
pub struct RenderItem {
    /// The item's id
    pub item_id: u16,
}

#[derive(Component)]
struct ItemTooltipPointer(Entity);

#[derive(Component)]
/// Put this component on any [`RenderItem`] that you don't want to have a tooltip on hover.
pub struct NoHoverTooltip;

#[derive(Component)]
struct ItemTooltip;

#[derive(Component, Reflect, Debug)]
/// A [`RenderItem`] with this will display this text instead of the item's name.
pub struct CustomHoverTooltip(String);

impl CustomHoverTooltip {
    /// Creates a new Hover Tooltip with this text
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

fn render_tooltips(
    mut commands: Commands,
    q_changed_interaction: Query<
        (Entity, &Interaction, &RenderItem, Option<&ItemTooltipPointer>),
        (Without<NoHoverTooltip>, Changed<Interaction>, Without<CustomHoverTooltip>),
    >,
    q_changed_interaction_custom: Query<
        (Entity, &Interaction, &CustomHoverTooltip, Option<&ItemTooltipPointer>),
        (Without<NoHoverTooltip>, Changed<Interaction>),
    >,
    q_any_item_tooltips: Query<&Interaction, With<ItemTooltipPointer>>,
    font: Res<DefaultFont>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
) {
    let mut spawned = false;
    for (ent, interaction, text, hovered_tooltip) in q_changed_interaction
        .iter()
        .map(|(ent, interaction, render_item, hovered_tooltip)| {
            let unlocalized_name = items.from_numeric_id(render_item.item_id).unlocalized_name();
            let item_name = lang.get_name_from_id(unlocalized_name).unwrap_or(unlocalized_name).to_owned();

            (ent, interaction, item_name, hovered_tooltip)
        })
        .chain(
            q_changed_interaction_custom
                .iter()
                .map(|(ent, interaction, custom, hovered_tooltip)| (ent, interaction, custom.0.clone(), hovered_tooltip)),
        )
    {
        if *interaction == Interaction::None {
            if let Some(ht) = hovered_tooltip {
                commands.entity(ht.0).insert(NeedsDespawned);
                commands.entity(ent).remove::<ItemTooltipPointer>();
            }
        } else {
            if spawned {
                continue;
            };

            if hovered_tooltip.is_some() {
                continue;
            }

            if q_any_item_tooltips.iter().any(|x| *x != Interaction::None) {
                // We only want one at a time
                continue;
            }

            spawned = true;

            let text_style = TextFont {
                font: font.0.clone_weak(),
                font_size: 24.0,
                ..Default::default()
            };

            let tt_ent = commands
                .spawn((
                    ItemTooltip,
                    Node {
                        position_type: PositionType::Absolute,
                        padding: UiRect::all(Val::Px(4.0)),
                        ..Default::default()
                    },
                    BackgroundColor(
                        Srgba {
                            red: 0.0,
                            green: 0.0,
                            blue: 0.0,
                            alpha: 0.95,
                        }
                        .into(),
                    ),
                    Name::new("Item Tooltip"),
                    GlobalZIndex(100),
                ))
                .with_children(|p| {
                    p.spawn((Text::new(text), text_style.clone()));
                })
                .set_parent(ent)
                .id();

            commands.entity(ent).insert(ItemTooltipPointer(tt_ent));
        }
    }
}

fn reposition_tooltips(
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_tooltip: Query<(&mut Node, &Parent), With<ItemTooltip>>,
    q_node: Query<(&GlobalTransform, &ComputedNode)>,
) {
    for (mut tt_node, parent) in q_tooltip.iter_mut() {
        let Ok(window) = q_windows.get_single() else {
            continue;
        };

        let Some(cursor_pos) = window.cursor_position() else {
            continue;
        };

        let Ok((g_trans, parent_node)) = q_node.get(parent.get()) else {
            continue;
        };

        let t = g_trans.translation();
        let bounds = Rect::from_center_size(Vec2::new(t.x, t.y), parent_node.size());
        let offset = cursor_pos - bounds.min;

        tt_node.left = Val::Px(offset.x + 5.0);
        tt_node.top = Val::Px(offset.y + 5.0);
    }
}

fn render_items(
    mut commands: Commands,
    items: Res<Registry<Item>>,
    q_changed_render_items: Query<(Entity, &RenderItem), Changed<RenderItem>>,
    item_atlas: Res<RenderedItemAtlas>,
    mut removed_render_items: RemovedComponents<RenderItem>,
) {
    for entity in removed_render_items.read() {
        if let Some(mut ecmds) = commands.get_entity(entity) {
            ecmds.remove::<(ImageNode, Interaction)>();
        }
    }

    for (changed_render_item_ent, render_item) in q_changed_render_items.iter() {
        commands.entity(changed_render_item_ent).insert((
            Interaction::default(),
            ImageNode {
                rect: Some(item_atlas.get_item_rect(items.from_numeric_id(render_item.item_id))),
                image: item_atlas.get_atlas_handle().clone_weak(),
                ..Default::default()
            },
        ));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Add systems prior to this if you are having 3d items rendered to the screen and you don't want a 1-frame delay
///
/// Use the `RenderItem` component to render an item in a ui component.
pub enum RenderItemSystemSet {
    /// Turn the `RenderItem` component into an actual UI component on your screen
    RenderItems,
}

pub(super) fn register(app: &mut App) {
    photo_booth::register(app);

    app.configure_sets(Update, RenderItemSystemSet::RenderItems.in_set(UiSystemSet::DoUi))
        .add_systems(
            Update,
            (render_items, render_tooltips, reposition_tooltips)
                .chain()
                .in_set(RenderItemSystemSet::RenderItems)
                .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
        )
        .register_type::<CustomHoverTooltip>();

    app.register_type::<RenderItem>();
}
