use bevy::{color::palettes::css, prelude::*, text::FontStyle};
use cosmos_core::{
    block::multiblock::reactor::{OpenReactorEvent, ReactorActive, Reactors},
    inventory::Inventory,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    prelude::{Structure, StructureBlock},
};

use crate::{
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed, InventorySide},
    ui::{components::window::GuiWindow, font::DefaultFont, OpenMenu},
};

#[derive(Component)]
struct ActiveText(StructureBlock);

fn create_ui(
    mut commands: Commands,
    mut evr_open_reactor: EventReader<OpenReactorEvent>,
    q_structure: Query<&Structure>,
    q_reactor_active: Query<&ReactorActive>,
    q_inventory: Query<Entity, (With<LocalPlayer>, With<Inventory>)>,
    font: Res<DefaultFont>,
) {
    for ev in evr_open_reactor.read() {
        let Ok(structure) = q_structure.get(ev.0.structure()) else {
            continue;
        };
        let Some(bd_ent) = structure.block_data(ev.0.coords()) else {
            continue;
        };

        let Ok(lp) = q_inventory.get_single() else {
            continue;
        };

        commands.entity(lp).insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));

        let active = structure.query_block_data(ev.0.coords(), &q_reactor_active).is_some();

        let mut fuel_slot_ent = None;

        let font = TextFont {
            font: font.0.clone_weak(),
            font_size: 24.0,
            ..Default::default()
        };

        commands
            .spawn((
                Name::new("Reactor UI"),
                OpenMenu::new(0),
                BorderColor(Color::BLACK),
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
                    width: Val::Px(300.0),
                    height: Val::Px(400.0),
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
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((ActiveText(ev.0), font, Text::new(if active { "ACTIVE" } else { "IDLE" })));

                    fuel_slot_ent = Some(
                        p.spawn(
                            (Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                ..Default::default()
                            }),
                        )
                        .id(),
                    );

                    p.spawn((
                        ReactorFuelStatusBar,
                        Node {
                            width: Val::Percent(90.0),
                            height: Val::Px(10.0),
                            margin: UiRect::vertical(Val::Px(50.0)),
                            ..Default::default()
                        },
                        BackgroundColor(css::LIME.into()),
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

#[derive(Component)]
struct ReactorFuelStatusBar;

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, create_ui.in_set(NetworkingSystemsSet::Between));
}
