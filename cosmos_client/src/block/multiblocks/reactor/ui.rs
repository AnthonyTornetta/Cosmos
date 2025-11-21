use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::multiblock::reactor::{
        ClientRequestChangeReactorStatus, OpenReactorMessage, Reactor, ReactorActive, ReactorFuel, ReactorFuelConsumption,
    },
    inventory::Inventory,
    netty::{
        client::LocalPlayer,
        sync::{
            events::client_event::NettyMessageWriter,
            mapping::{Mappable, NetworkMapping},
        },
    },
    prelude::{Structure, StructureBlock},
    registry::Registry,
    state::GameState,
};

use crate::{
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed, InventorySide},
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            button::{ButtonMessage, ButtonStyles, CosmosButton},
            window::GuiWindow,
        },
        font::DefaultFont,
    },
};

#[derive(Component)]
struct ActiveText(StructureBlock);

#[derive(Component)]
struct ReactorBlockReference(StructureBlock);

#[derive(Component)]
pub struct ReactorPowerGenStats;

fn create_ui(
    mut commands: Commands,
    mut evr_open_reactor: MessageReader<OpenReactorMessage>,
    q_structure: Query<&Structure>,
    q_reactor_active: Query<&ReactorActive>,
    q_inventory: Query<Entity, (With<LocalPlayer>, With<Inventory>)>,
    font: Res<DefaultFont>,
) {
    for ev in evr_open_reactor.read() {
        let Ok(structure) = q_structure.get(ev.0.structure()) else {
            error!("No structure!");
            continue;
        };
        let Some(bd_ent) = structure.block_data(ev.0.coords()) else {
            error!("No block data ent!");
            continue;
        };

        let Ok(lp) = q_inventory.single() else {
            error!("No block inventory data!");
            continue;
        };

        commands.entity(lp).insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));

        let active = structure.query_block_data(ev.0.coords(), &q_reactor_active).is_some();

        let mut fuel_slot_ent = None;

        let font = TextFont {
            font: font.0.clone(),
            font_size: 24.0,
            ..Default::default()
        };

        commands
            .spawn((
                Name::new("Reactor UI"),
                OpenMenu::new(0),
                BorderColor::all(Color::BLACK),
                GuiWindow {
                    title: "Reactor".into(),
                    body_styles: Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Node {
                    right: Val::Px(200.0),
                    left: Val::Auto,
                    top: Val::Px(100.0),
                    width: Val::Px(350.0),
                    height: Val::Px(500.0),
                    position_type: PositionType::Absolute,
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                    Node {
                        flex_grow: 1.0,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        ActiveText(ev.0),
                        font.clone(),
                        Node {
                            margin: UiRect::bottom(Val::Px(50.0)),
                            ..Default::default()
                        },
                        Text::new(if active { "ACTIVE" } else { "IDLE" }),
                    ));

                    fuel_slot_ent = Some(
                        p.spawn((
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                ..Default::default()
                            },
                            Name::new("Fuel Slot"),
                        ))
                        .id(),
                    );

                    p.spawn((
                        Node {
                            width: Val::Percent(90.0),
                            height: Val::Px(10.0),
                            margin: UiRect::vertical(Val::Px(50.0)),
                            ..Default::default()
                        },
                        BackgroundColor(css::GRAY.into()),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            ReactorFuelStatusBar,
                            ReactorBlockReference(ev.0),
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..Default::default()
                            },
                            BackgroundColor(css::LIME.into()),
                        ));
                    });

                    p.spawn((
                        ReactorBlockReference(ev.0),
                        ToggleReactorBtn,
                        CosmosButton {
                            text: Some((
                                if active { "DEACTIVATE" } else { "ACTIVATE" }.into(),
                                font.clone(),
                                Default::default(),
                            )),
                            button_styles: Some(ButtonStyles {
                                background_color: css::GREY.into(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        Node {
                            padding: UiRect::new(Val::Px(8.0), Val::Px(4.0), Val::Px(8.0), Val::Px(4.0)),
                            ..Default::default()
                        },
                    ))
                    .observe(on_click_toggle);

                    p.spawn((
                        ReactorPowerGenStats,
                        ReactorBlockReference(ev.0),
                        Node {
                            margin: UiRect::top(Val::Px(40.0)),
                            ..Default::default()
                        },
                        Text::new(""),
                        font.clone(),
                    ));
                });
            });

        commands
            .entity(bd_ent)
            .insert(InventoryNeedsDisplayed::Custom(CustomInventoryRender::new(vec![(
                0,
                fuel_slot_ent.expect("Set above"),
            )])));

        break;
    }
}

fn on_click_toggle(
    ev: On<ButtonMessage>,
    q_active: Query<(), With<ReactorActive>>,
    q_structure: Query<&Structure>,
    q_ref: Query<&ReactorBlockReference>,
    mut nevw: NettyMessageWriter<ClientRequestChangeReactorStatus>,
    mapping: Res<NetworkMapping>,
) {
    let Ok(reference) = q_ref.get(ev.0) else {
        return;
    };

    let Ok(structure) = q_structure.get(reference.0.structure()) else {
        return;
    };

    let active = structure.query_block_data(reference.0.coords(), &q_active).is_none();

    let Ok(mapped_sb) = reference.0.map_to_server(&mapping) else {
        return;
    };

    nevw.write(ClientRequestChangeReactorStatus { active, block: mapped_sb });
}

#[derive(Component, Debug)]
struct ToggleReactorBtn;

#[derive(Component)]
struct ReactorFuelStatusBar;

fn maintain_active_text(
    q_active: Query<(), With<ReactorActive>>,
    q_structure: Query<&Structure>,
    mut q_active_text: Query<(&mut Text, &ActiveText)>,
    mut q_btn: Query<&mut CosmosButton, With<ToggleReactorBtn>>,
) {
    for (mut txt, active_text) in q_active_text.iter_mut() {
        let Ok(s) = q_structure.get(active_text.0.structure()) else {
            continue;
        };

        if s.query_block_data(active_text.0.coords(), &q_active).is_some() {
            if txt.0 != "ACTIVE" {
                txt.0 = "ACTIVE".into();
                if let Ok(mut btn) = q_btn.single_mut() {
                    btn.text.as_mut().expect("No text?").0 = "DEACTIVATE".into();
                }
            }
        } else if txt.0 != "IDLE" {
            txt.0 = "IDLE".into();
            if let Ok(mut btn) = q_btn.single_mut() {
                btn.text.as_mut().expect("No text?").0 = "ACTIVATE".into();
            }
        }
    }
}

fn update_status_bar(
    mut q_status_bar: Query<(&mut Node, &ReactorBlockReference), With<ReactorFuelStatusBar>>,
    q_structure: Query<&Structure>,
    q_fuel_consumption: Query<&ReactorFuelConsumption>,
    fuels: Res<Registry<ReactorFuel>>,
) {
    for (mut node, reactor_ref) in q_status_bar.iter_mut() {
        let Ok(structure) = q_structure.get(reactor_ref.0.structure()) else {
            continue;
        };

        let Some(fuel_cons) = structure.query_block_data(reactor_ref.0.coords(), &q_fuel_consumption) else {
            node.width = Val::Px(0.0);
            continue;
        };

        let fuel = fuels.from_numeric_id(fuel_cons.fuel_id);
        node.width = Val::Percent(100.0 - (fuel_cons.secs_spent / fuel.lasts_for.as_secs_f32()).min(1.0) * 100.0);
    }
}

fn update_generation_stats(
    mut q_status_bar: Query<(&mut Text, &ReactorBlockReference), With<ReactorPowerGenStats>>,
    q_structure: Query<&Structure>,
    q_fuel_consumption: Query<(&Reactor, &ReactorFuelConsumption), With<ReactorActive>>,
    fuels: Res<Registry<ReactorFuel>>,
) {
    for (mut txt, reactor_ref) in q_status_bar.iter_mut() {
        let Ok(structure) = q_structure.get(reactor_ref.0.structure()) else {
            continue;
        };

        let Some((reactor, fuel_cons)) = structure.query_block_data(reactor_ref.0.coords(), &q_fuel_consumption) else {
            let text = "Generating 0 kW";
            if txt.0 != text {
                txt.0 = text.into();
            }
            continue;
        };

        let fuel = fuels.from_numeric_id(fuel_cons.fuel_id);
        let text = format!("Generating {} kW", fuel.multiplier * reactor.power_per_second());
        if txt.0 != text {
            txt.0 = text;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            create_ui.before(UiSystemSet::PreDoUi),
            maintain_active_text,
            update_status_bar,
            update_generation_stats,
        )
            .run_if(in_state(GameState::Playing)),
    );
}
