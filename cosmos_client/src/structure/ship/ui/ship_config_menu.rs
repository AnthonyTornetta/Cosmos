use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{ecs::NeedsDespawned, netty::client::LocalPlayer, state::GameState, structure::ship::pilot::Pilot};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    structure::ship::ui::{details::ShipDetailsUi, ship_systems::ShipSystemsUi},
    ui::{
        OpenMenu,
        components::{
            tabbed_view::{Tab, TabbedView},
            window::GuiWindow,
        },
    },
};

#[derive(Component)]
struct OpenShipMenu;

fn open_config_menu(
    q_open_menu: Query<Entity, With<OpenShipMenu>>,
    inputs: InputChecker,
    mut commands: Commands,
    q_pilot: Query<&Pilot, With<LocalPlayer>>,
) {
    let toggle_menu = inputs.check_just_pressed(CosmosInputs::OpenShipConfiguration);

    if toggle_menu && !q_open_menu.is_empty() {
        if let Ok(open) = q_open_menu.single() {
            info!("Closing!");
            commands.entity(open).insert(NeedsDespawned);
        }
        return;
    }

    let Ok(pilot) = q_pilot.single() else {
        if let Ok(open) = q_open_menu.single() {
            info!("Closing!");
            commands.entity(open).insert(NeedsDespawned);
        }
        return;
    };

    if !toggle_menu {
        return;
    }

    info!("Opening!");
    commands
        .spawn((
            OpenShipMenu,
            Name::new("Ship Config Menu"),
            GuiWindow {
                title: "Ship".into(),
                body_styles: Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
            OpenMenu::new(0),
            BorderColor(css::DARK_GREY.into()),
            Node {
                margin: UiRect::all(Val::Auto),
                position_type: PositionType::Absolute,
                width: Val::Px(600.0),
                height: Val::Px(800.0),
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                TabbedView { ..Default::default() },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((ShipSystemsUi::new(pilot.entity), Tab::new("Systems")));
                p.spawn((ShipDetailsUi::new(pilot.entity), Tab::new("Details")));
                p.spawn((Text::new("quests"), Tab::new("Quests")));
            });
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, open_config_menu.run_if(in_state(GameState::Playing)));
}
