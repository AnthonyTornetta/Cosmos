use bevy::prelude::*;
use cosmos_core::{
    block::multiblock::prelude::{ClientFriendlyShipyardState, ShowShipyardUi},
    prelude::{Structure, StructureBlock},
};

use crate::ui::{OpenMenu, components::window::GuiWindow};

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

fn create_shipyard_ui(commands: &mut Commands, state: Option<&ClientFriendlyShipyardState>, block: StructureBlock) {
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
                p.spawn((Text::new("Select Blueprint")));
            }
            Some(ClientFriendlyShipyardState::Paused(p)) => {}
            Some(ClientFriendlyShipyardState::Building(b)) => {}
            Some(ClientFriendlyShipyardState::Deconstructing(e)) => {}
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_open_shipyard);
}
