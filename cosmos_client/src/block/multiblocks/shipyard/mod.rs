use bevy::prelude::*;
use cosmos_core::{
    block::multiblock::prelude::{ClientFriendlyShipyardState, ShowShipyardUi},
    inventory::Inventory,
    item::{Item, usable::blueprint::BlueprintItemData},
    netty::client::LocalPlayer,
    prelude::{Structure, StructureBlock},
    registry::identifiable::Identifiable,
};

use crate::{
    inventory::InventoryNeedsDisplayed,
    ui::{
        OpenMenu,
        components::{
            button::{ButtonEvent, CosmosButton},
            window::GuiWindow,
        },
        item_renderer::RenderItem,
    },
};

fn on_open_shipyard(
    q_structure: Query<&Structure>,
    mut nevr_open_shipyard: EventReader<ShowShipyardUi>,
    q_shipyard_state: Query<&ClientFriendlyShipyardState>,
    commands: Commands,
) {
    let Some(ev) = nevr_open_shipyard.read().next() else {
        return;
    };

    let Ok(structure) = q_structure.get(ev.shipyard_block.structure()) else {
        return;
    };

    let state = structure.query_block_data(ev.shipyard_block.coords(), &q_shipyard_state);
}

fn create_shipyard_ui(
    commands: &mut Commands,
    state: Option<&ClientFriendlyShipyardState>,
    block: StructureBlock,
    inventory: &Inventory,
    q_blueprint_data: &Query<&BlueprintItemData>,
    blueprint: &Item,
    q_inventory: Query<Entity, With<LocalPlayer>>,
) {
    let Ok(inv) = q_inventory.single() else {
        return;
    };

    commands
        .entity(inv)
        .insert(InventoryNeedsDisplayed::Normal(crate::inventory::InventorySide::Left));

    commands
        .spawn((
            OpenMenu::new(0),
            BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
            Node {
                width: Val::Px(800.0),
                height: Val::Px(800.0),
                margin: UiRect {
                    // Centers it vertically
                    top: Val::Auto,
                    bottom: Val::Auto,
                    // Aligns it 100px from the right
                    left: Val::Auto,
                    right: Val::Px(100.0),
                },
                ..Default::default()
            },
            GuiWindow {
                title: "Shipyard".into(),
                body_styles: Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|p| match state {
            None => {
                p.spawn((Text::new("Insert Blueprint")));

                for bp in inventory
                    .iter()
                    .flatten()
                    .filter(|i| i.item_id() == blueprint.id())
                    .flat_map(|i| i.data_entity().and_then(|e| q_blueprint_data.get(e).ok()))
                {
                    p.spawn((CosmosButton { ..Default::default() }))
                        .observe(|ev: Trigger<ButtonEvent>| info!("{ev:?}"))
                        .with_children(|p| {
                            p.spawn((
                                RenderItem { item_id: blueprint.id() },
                                Node {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    ..Default::default()
                                },
                            ));
                        });
                }
            }
            Some(ClientFriendlyShipyardState::Paused(p)) => {}
            Some(ClientFriendlyShipyardState::Building(b)) => {}
            Some(ClientFriendlyShipyardState::Deconstructing(e)) => {}
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_open_shipyard);
}
