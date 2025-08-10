use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::{faction::FactionId, netty::client::LocalPlayer, state::GameState};

use crate::{
    create_button_event,
    ui::{
        components::button::{ButtonEvent, CosmosButton, register_button},
        font::DefaultFont,
    },
};

#[derive(Component)]
#[require(Node)]
pub struct FactionDisplay;

create_button_event!(CreateFaction);

fn render_no_faction(p: &mut RelatedSpawnerCommands<ChildOf>, font: &DefaultFont) {
    p.spawn(
        (Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }),
    )
    .with_children(|p| {
        p.spawn((
            Text::new("No Faction"),
            TextFont {
                font_size: 24.0,
                font: font.get(),
                ..Default::default()
            },
        ));

        p.spawn(
            (CosmosButton::<CreateFaction> {
                text: Some((
                    "Create Faction".into(),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    Default::default(),
                )),
                ..Default::default()
            }),
        );
    });
}

fn render_faction_display(
    mut commands: Commands,
    q_added_fac_display: Query<Entity, Added<FactionDisplay>>,
    q_faction: Query<&FactionId, With<LocalPlayer>>,
    font: Res<DefaultFont>,
) {
    for ent in q_added_fac_display.iter() {
        commands.entity(ent).insert((Name::new("Faction Display"),)).with_children(|p| {
            if let Ok(fac_id) = q_faction.single() {
            } else {
                render_no_faction(p, &font);
            }
        });
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<CreateFaction>(app);
    app.add_systems(Update, render_faction_display.run_if(in_state(GameState::Playing)));
}
