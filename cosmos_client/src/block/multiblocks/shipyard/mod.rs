use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::{
    block::{
        Block,
        data::BlockData,
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
            scollable_container::ScrollBox,
            window::GuiWindow,
        },
        font::DefaultFont,
    },
};

fn on_change_shipyard_state(
    q_shipyard_state: Query<(&ClientFriendlyShipyardState, &BlockData), Changed<ClientFriendlyShipyardState>>,
    mut removed_states: RemovedComponents<ClientFriendlyShipyardState>,
    q_block_data: Query<&BlockData>,
    q_opened_shipyard_ui: Query<(Entity, &OpenedShipyard)>,
    mut commands: Commands,
    q_inventory: Query<&Inventory, With<LocalPlayer>>,
    q_blueprint_data: Query<&BlueprintItemData>,
    items: Res<Registry<Item>>,
    font: Res<DefaultFont>,
    factions: Res<Factions>,
    blocks: Res<Registry<Block>>,
    lang: Res<Lang<Block>>,
) {
    for (state, block) in q_shipyard_state
        .iter()
        .map(|(s, b)| (Some(s), b))
        .chain(removed_states.read().flat_map(|e| q_block_data.get(e).map(|d| (None, d))))
    {
        let Ok((ent, opened)) = q_opened_shipyard_ui.single() else {
            return;
        };

        if opened.0 != block.identifier.block {
            continue;
        }

        let Some(blueprint) = items.from_id("cosmos:blueprint") else {
            continue;
        };

        let Ok(player_inv) = q_inventory.single() else {
            return;
        };

        commands.entity(ent).despawn_related::<Children>().with_children(|p| {
            create_shipyard_ui(
                p,
                state,
                block.identifier.block,
                &q_blueprint_data,
                blueprint,
                &font,
                &factions,
                &blocks,
                &lang,
                player_inv,
            );
        });
    }
}

#[derive(Component)]
struct OpenedShipyard(StructureBlock);

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
        .with_children(|p| {
            p.spawn((
                OpenedShipyard(ev.shipyard_block),
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                create_shipyard_ui(
                    p,
                    state,
                    ev.shipyard_block,
                    &q_blueprint_data,
                    blueprint,
                    &font,
                    &factions,
                    &blocks,
                    &lang,
                    inventory,
                );
            });
        });
}

fn create_shipyard_ui(
    p: &mut RelatedSpawnerCommands<ChildOf>,
    state: Option<&ClientFriendlyShipyardState>,
    block: StructureBlock,
    q_blueprint_data: &Query<&BlueprintItemData>,
    blueprint: &Item,
    font: &DefaultFont,
    factions: &Factions,
    blocks: &Registry<Block>,
    lang: &Lang<Block>,
    player_inv: &Inventory,
) {
    match state {
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

            for (slot, bp) in player_inv
                .iter()
                .enumerate()
                .flat_map(|(slot, item)| item.as_ref().map(|item| (slot, item)))
                .filter(|(_, i)| i.item_id() == blueprint.id())
                .flat_map(|(slot, i)| i.data_entity().and_then(|e| q_blueprint_data.get(e).ok().map(|d| (slot, d))))
                .filter(|(_, bp)| bp.blueprint_type == BlueprintType::Ship)
            {
                p.spawn((
                    Node {
                        width: Val::Percent(32.0),
                        margin: UiRect::all(Val::Percent(1.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Column,
                        height: Val::Px(300.0),
                        ..Default::default()
                    },
                    BorderColor(css::AQUA.into()),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new(bp.name.clone()),
                        TextFont {
                            font: font.get(),
                            font_size: 24.0,
                            ..Default::default()
                        },
                    ));

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
                            width: Val::Percent(100.0),
                            height: Val::Px(50.0),
                            margin: UiRect::top(Val::Auto),
                            border: UiRect::all(Val::Px(2.0)),
                            ..Default::default()
                        },
                        BorderColor(css::YELLOW.into()),
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
                });
            }
        }
        Some(ClientFriendlyShipyardState::Paused(d)) => {
            p.spawn((
                Text::new("Building (Paused)"),
                TextFont {
                    font_size: 32.0,
                    font: font.get(),
                    ..Default::default()
                },
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..Default::default()
                },
            ));

            p.spawn((
                ScrollBox::default(),
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
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
                            font_size: 24.0,
                            font: font.get(),
                            ..Default::default()
                        },
                        Node {
                            margin: UiRect::all(Val::Px(25.0)),
                            ..Default::default()
                        },
                    ));
                }
            });

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
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    margin: UiRect::all(Val::Auto),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                BorderColor(css::YELLOW.into()),
            ))
            .observe(
                move |_trigger: Trigger<ButtonEvent>, mut nevw_change_shipyard_state: NettyEventWriter<ClientSetShipyardState>| {
                    info!("Resume shipyard!");
                    nevw_change_shipyard_state.write(ClientSetShipyardState::Unpause { controller: block });
                },
            );
        }
        Some(ClientFriendlyShipyardState::Building(b)) => {
            p.spawn((
                Text::new("Building"),
                TextFont {
                    font_size: 32.0,
                    font: font.get(),
                    ..Default::default()
                },
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    ..Default::default()
                },
            ));

            p.spawn((
                ScrollBox::default(),
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
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
                            font_size: 24.0,
                            font: font.get(),
                            ..Default::default()
                        },
                        Node {
                            margin: UiRect::all(Val::Px(25.0)),
                            ..Default::default()
                        },
                    ));
                }
            });

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
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    margin: UiRect::all(Val::Auto),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                BorderColor(css::YELLOW.into()),
            ))
            .observe(
                move |_trigger: Trigger<ButtonEvent>, mut nevw_change_shipyard_state: NettyEventWriter<ClientSetShipyardState>| {
                    info!("Pause shipyard!");
                    nevw_change_shipyard_state.write(ClientSetShipyardState::Pause { controller: block });
                },
            );
        }
        Some(ClientFriendlyShipyardState::Deconstructing(e)) => {
            p.spawn(Text::new(format!("DECONSTRUCTING TODO {e:?}")));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_open_shipyard, on_change_shipyard_state).run_if(in_state(GameState::Playing)),
    );
}
