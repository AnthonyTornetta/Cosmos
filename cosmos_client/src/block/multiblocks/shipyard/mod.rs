use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::{
        Block,
        multiblock::prelude::{ClientFriendlyShipyardState, ClientSetShipyardState, SetShipyardBlueprint, ShowShipyardUi},
    },
    faction::Factions,
    inventory::Inventory,
    item::{Item, usable::blueprint::BlueprintItemData},
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    prelude::{Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::blueprint::{BlueprintAuthor, BlueprintType},
};

use crate::{
    inventory::InventoryNeedsDisplayed,
    lang::Lang,
    ui::{
        OpenMenu,
        components::{
            button::{ButtonEvent, CosmosButton},
            window::GuiWindow,
        },
        font::DefaultFont,
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
    font: Res<DefaultFont>,
    factions: Res<Factions>,
    blocks: Res<Registry<Block>>,
    lang: Res<Lang<Block>>,
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

    create_shipyard_ui(
        &mut commands,
        state,
        ev.shipyard_block,
        &q_blueprint_data,
        blueprint,
        &q_inventory,
        &font,
        &factions,
        &blocks,
        &lang,
    );
}

fn create_shipyard_ui(
    commands: &mut Commands,
    state: Option<&ClientFriendlyShipyardState>,
    block: StructureBlock,
    q_blueprint_data: &Query<&BlueprintItemData>,
    blueprint: &Item,
    q_inventory: &Query<(Entity, &Inventory), With<LocalPlayer>>,
    font: &DefaultFont,
    factions: &Factions,
    blocks: &Registry<Block>,
    lang: &Lang<Block>,
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
                p.spawn((
                    Text::new("Select Blueprint"),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Node {
                        margin: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(10.0), Val::Px(20.0)),
                        ..Default::default()
                    },
                ));

                for (slot, bp) in inventory
                    .iter()
                    .enumerate()
                    .flat_map(|(slot, item)| item.as_ref().map(|item| (slot, item)))
                    .filter(|(_, i)| i.item_id() == blueprint.id())
                    .flat_map(|(slot, i)| i.data_entity().and_then(|e| q_blueprint_data.get(e).ok().map(|d| (slot, d))))
                    .filter(|(_, bp)| bp.blueprint_type == BlueprintType::Ship)
                {
                    p.spawn((Node { ..Default::default() },)).with_children(|p| {
                        p.spawn((
                            CustomHoverTooltip::new(bp.name.clone()),
                            RenderItem { item_id: blueprint.id() },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                border: UiRect::all(Val::Px(2.0)),
                                margin: UiRect::all(Val::Px(16.0)),
                                ..Default::default()
                            },
                            BackgroundColor(css::GREY.into()),
                            BorderColor(css::AQUA.into()),
                        ));

                        p.spawn((
                            Text::new(bp.name.clone()),
                            TextFont {
                                font: font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));
                    });

                    match &bp.author {
                        BlueprintAuthor::Player { name, id: _ } => {
                            p.spawn((
                                Text::new(format!("Creator: {}", name.clone())),
                                TextFont {
                                    font: font.get(),
                                    font_size: 20.0,
                                    ..Default::default()
                                },
                            ));
                        }
                        BlueprintAuthor::Faction(f) => {
                            if let Some(fac) = factions.from_id(f) {
                                p.spawn((
                                    Text::new(format!("Creator: {}", fac.name())),
                                    TextFont {
                                        font: font.get(),
                                        font_size: 20.0,
                                        ..Default::default()
                                    },
                                ));
                            }
                        }
                        BlueprintAuthor::Server => {}
                    }

                    p.spawn((
                        Name::new("Blueprint btn"),
                        CosmosButton {
                            text: Some((
                                "Construct".into(),
                                TextFont {
                                    font: font.get(),
                                    font_size: 20.0,
                                    ..Default::default()
                                },
                                Default::default(),
                            )),
                            ..Default::default()
                        },
                        Node {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                    ))
                    .observe(
                        move |ev: Trigger<ButtonEvent>, mut nevw_set_blueprint: NettyEventWriter<SetShipyardBlueprint>| {
                            info!("Setting shipyard blueprint ({ev:?})");
                            nevw_set_blueprint.write(SetShipyardBlueprint {
                                shipyard_block: block,
                                blueprint_slot: slot as u32,
                            });
                        },
                    );
                }
            }
            Some(ClientFriendlyShipyardState::Paused(d)) => {
                p.spawn((
                    Text::new("Building (Paused)"),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    Node { ..Default::default() },
                ));

                p.spawn((
                    CosmosButton {
                        text: Some((
                            "Resume".into(),
                            TextFont {
                                font_size: 20.0,
                                font: font.get(),
                                ..Default::default()
                            },
                            Default::default(),
                        )),
                        ..Default::default()
                    },
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        ..Default::default()
                    },
                ))
                .observe(
                    move |_trigger: Trigger<ButtonEvent>, mut nevw_change_shipyard_state: NettyEventWriter<ClientSetShipyardState>| {
                        nevw_change_shipyard_state.write(ClientSetShipyardState::Unpause { controller: block });
                    },
                );

                // Sort by amt required
                let mut items_needed = d
                    .remaining_blocks
                    .iter()
                    .map(|(a, b)| (blocks.from_numeric_id(*a), *b))
                    .collect::<Vec<_>>();

                items_needed.sort_unstable_by_key(|x| !x.1);

                for (block, qty) in items_needed {
                    p.spawn((
                        Text::new(format!("{} - {}", lang.get_name_or_unlocalized(block), qty)),
                        TextFont {
                            font_size: 20.0,
                            font: font.get(),
                            ..Default::default()
                        },
                        Node { ..Default::default() },
                    ));
                }
            }
            Some(ClientFriendlyShipyardState::Building(b)) => {
                p.spawn((
                    Text::new("Building"),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    Node { ..Default::default() },
                ));

                p.spawn((
                    CosmosButton {
                        text: Some((
                            "Pause".into(),
                            TextFont {
                                font_size: 20.0,
                                font: font.get(),
                                ..Default::default()
                            },
                            Default::default(),
                        )),
                        ..Default::default()
                    },
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        ..Default::default()
                    },
                ))
                .observe(
                    move |_trigger: Trigger<ButtonEvent>, mut nevw_change_shipyard_state: NettyEventWriter<ClientSetShipyardState>| {
                        nevw_change_shipyard_state.write(ClientSetShipyardState::Pause { controller: block });
                    },
                );

                // Sort by amt required
                let mut items_needed = b
                    .remaining_blocks
                    .iter()
                    .map(|(a, b)| (blocks.from_numeric_id(*a), *b))
                    .collect::<Vec<_>>();

                items_needed.sort_unstable_by_key(|x| !x.1);

                for (block, qty) in items_needed {
                    p.spawn((
                        Text::new(format!("{} - {}", lang.get_name_or_unlocalized(block), qty)),
                        TextFont {
                            font_size: 20.0,
                            font: font.get(),
                            ..Default::default()
                        },
                        Node { ..Default::default() },
                    ));
                }
            }
            Some(ClientFriendlyShipyardState::Deconstructing(e)) => {
                p.spawn(Text::new(format!("DECONSTRUCTING TODO {e:?}")));
            }
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_open_shipyard.run_if(in_state(GameState::Playing)));
}
