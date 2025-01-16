//! Renders items as 3d models at based off the RenderItem present in a UI element

use bevy::prelude::*;
use cosmos_core::{item::Item, registry::Registry};
use photo_booth::RenderedItemAtlas;

use super::UiSystemSet;

pub mod photo_booth;

#[derive(Debug, Component, Reflect)]
/// Put this onto a UI element to render a 3D item there
pub struct RenderItem {
    /// The item's id
    pub item_id: u16,
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
            ecmds.remove::<ImageNode>();
        }
    }

    for (changed_render_item_ent, render_item) in q_changed_render_items.iter() {
        commands.entity(changed_render_item_ent).insert(ImageNode {
            rect: Some(item_atlas.get_item_rect(items.from_numeric_id(render_item.item_id))),
            image: item_atlas.get_atlas_handle().clone_weak(),
            ..Default::default()
        });
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
        .add_systems(Update, render_items.chain().in_set(RenderItemSystemSet::RenderItems));

    app.register_type::<RenderItem>();
}
