use bevy::prelude::*;
use cosmos_core::{ecs::NeedsDespawned, state::GameState};

use crate::ui::font::DefaultFont;

#[derive(Event, Debug)]
pub struct ShowError {
    pub text: String,
}

impl ShowError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { text: message.into() }
    }
}

#[derive(Component, Debug)]
struct PopupsList;

#[derive(Component, Debug)]
struct Error(f32);

fn init_error_list(mut commands: Commands) {
    commands.spawn((
        Name::new("Error List"),
        PopupsList,
        Node {
            top: Val::Px(50.0),
            right: Val::Px(0.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
    ));
}

const WIDTH: f32 = 500.0;

fn show_error(
    font: Res<DefaultFont>,
    q_errors: Query<Entity, With<PopupsList>>,
    mut commands: Commands,
    mut evr_error: EventReader<ShowError>,
) {
    for ev in evr_error.read() {
        let Ok(ent) = q_errors.single() else {
            return;
        };

        commands.entity(ent).with_children(|p| {
            p.spawn((
                Error(0.0),
                Node {
                    width: Val::Px(WIDTH),
                    height: Val::Px(150.0),
                    margin: UiRect::all(Val::Px(30.0)),
                    ..Default::default()
                },
                BackgroundColor(
                    Srgba {
                        red: 1.0,
                        green: 0.3,
                        blue: 0.3,
                        alpha: 0.7,
                    }
                    .into(),
                ),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new(ev.text.clone()),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                ));
            });
        });
    }
}

fn tick_error(mut commands: Commands, mut q_error: Query<(Entity, &mut Node, &mut Error)>, time: Res<Time>) {
    for (entity, mut node, mut err) in q_error.iter_mut() {
        err.0 += time.delta_secs();
        let left = (err.0 - 10.0).max(0.0).powf(4.0);
        node.left = Val::Px(left);

        if left > 2.0 * WIDTH {
            commands.entity(entity).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ShowError>()
        .add_systems(Update, (show_error, tick_error).chain())
        .add_systems(OnEnter(GameState::Playing), init_error_list);
}
