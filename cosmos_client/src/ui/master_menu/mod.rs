use bevy::prelude::*;
use cosmos_core::{ecs::NeedsDespawned, netty::client::LocalPlayer, state::GameState, structure::ship::pilot::Pilot};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        OpenMenu,
        components::{
            tabbed_view::{Tab, TabbedView},
            window::GuiWindow,
        },
        font::DefaultFont,
        master_menu::{faction::FactionDisplay, quest::QuestDisplay},
    },
};

mod faction;
mod quest;

#[derive(Component)]
struct OpenMasterMenu;

fn toggle_menu(
    q_piloting: Query<(), (With<LocalPlayer>, With<Pilot>)>,
    q_open_menu: Query<Entity, With<OpenMasterMenu>>,
    mut commands: Commands,
    inputs: InputChecker,
) {
    if !inputs.check_just_pressed(CosmosInputs::OpenShipConfiguration) {
        return;
    }

    if !q_piloting.is_empty() {
        return;
    }

    if let Ok(ent) = q_open_menu.single() {
        commands.entity(ent).insert(NeedsDespawned);
        return;
    }

    commands
        .spawn((
            OpenMasterMenu,
            GuiWindow {
                title: "Datapad".into(),
                body_styles: Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
            Node {
                margin: UiRect::all(Val::Auto),
                position_type: PositionType::Absolute,
                width: Val::Px(800.0),
                height: Val::Px(800.0),
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            OpenMenu::new(0),
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
                p.spawn((
                    Tab::new("Missions"),
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    QuestDisplay,
                ));
                p.spawn((
                    Tab::new("Faction"),
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    FactionDisplay,
                ));
            });
        });
}

pub(super) fn register(app: &mut App) {
    faction::register(app);
    quest::register(app);

    app.add_systems(Update, toggle_menu.run_if(in_state(GameState::Playing)));
}
