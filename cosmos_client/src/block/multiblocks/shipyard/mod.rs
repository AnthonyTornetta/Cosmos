use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::multiblock::prelude::{ClientFriendlyShipyardState, SetShipyardBlueprint, ShowShipyardUi},
    inventory::Inventory,
    item::{Item, usable::blueprint::BlueprintItemData},
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    prelude::{Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
    structure::blueprint::BlueprintType,
};

use crate::{
    inventory::InventoryNeedsDisplayed,
    ui::{
        OpenMenu,
        components::{
            button::{ButtonEvent, CosmosButton},
            window::GuiWindow,
        },
        item_renderer::{CustomHoverTooltip, RenderItem},
    },
};

fn on_open_shipyard(
    q_structure: Query<&Structure>,
    mut nevr_open_shipyard: EventReader<ShowShipyardUi>,
    q_shipyard_state: Query<&ClientFriendlyShipyardState>,
    q_inventory: Query<(Entity, &Inventory), With<LocalPlayer>>,
    q_blueprint_data: Query<&BlueprintItemData>,
    items: Res<Registry<Item>>,
    mut commands: Commands,
) {
    let Some(ev) = nevr_open_shipyard.read().next() else {
        return;
    };

    let Ok(structure) = q_structure.get(ev.shipyard_block.structure()) else {
        return;
    };

    let Some(blueprint) = items.from_id("cosmos:blueprint") else {
        return;
    };

    let state = structure.query_block_data(ev.shipyard_block.coords(), &q_shipyard_state);

    create_shipyard_ui(&mut commands, state, ev.shipyard_block, &q_blueprint_data, blueprint, &q_inventory);
}

fn create_shipyard_ui(
    commands: &mut Commands,
    state: Option<&ClientFriendlyShipyardState>,
    block: StructureBlock,
    q_blueprint_data: &Query<&BlueprintItemData>,
    blueprint: &Item,
    q_inventory: &Query<(Entity, &Inventory), With<LocalPlayer>>,
) {
    let Ok((inv, inventory)) = q_inventory.single() else {
        return;
    };

    commands
        .entity(inv)
        .insert(InventoryNeedsDisplayed::Normal(crate::inventory::InventorySide::Left));

    commands
        .spawn((
            Name::new("Shipyard UI"),
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
                p.spawn((Text::new("Select Blueprint")));

                for (slot, bp) in inventory
                    .iter()
                    .enumerate()
                    .flat_map(|(slot, item)| item.as_ref().map(|item| (slot, item)))
                    .filter(|(_, i)| i.item_id() == blueprint.id())
                    .flat_map(|(slot, i)| i.data_entity().and_then(|e| q_blueprint_data.get(e).ok().map(|d| (slot, d))))
                    .filter(|(_, bp)| bp.blueprint_type == BlueprintType::Ship)
                {
                    p.spawn((Name::new("Blueprint btn"), CosmosButton { ..Default::default() }))
                        .observe(
                            move |ev: Trigger<ButtonEvent>, mut nevw_set_blueprint: NettyEventWriter<SetShipyardBlueprint>| {
                                info!("Setting shipyard blueprint ({ev:?})");
                                nevw_set_blueprint.write(SetShipyardBlueprint {
                                    shipyard_block: block,
                                    blueprint_slot: slot as u32,
                                });
                            },
                        )
                        .with_children(|p| {
                            p.spawn((
                                CustomHoverTooltip::new(bp.name.clone()),
                                RenderItem { item_id: blueprint.id() },
                                Node {
                                    width: Val::Px(128.0),
                                    height: Val::Px(128.0),
                                    border: UiRect::all(Val::Px(2.0)),
                                    margin: UiRect::all(Val::Px(16.0)),
                                    ..Default::default()
                                },
                                BackgroundColor(css::GREY.into()),
                                BorderColor(css::AQUA.into()),
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
