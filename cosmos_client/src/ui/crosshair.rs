use bevy::prelude::*;

use crate::state::game_state::GameState;

fn add_crosshair(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            color: Color::NONE.into(),

            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(NodeBundle {
                    image: asset_server.load("images/ui/crosshair.png").into(),
                    style: Style {
                        size: Size::new(Val::Px(8.0), Val::Px(8.0)),
                        position: UiRect::new(
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                            Val::Px(0.0),
                        ),
                        ..default()
                    },

                    // color: Color::NONE.into(),
                    ..default()
                })
                .insert(Crosshair)
                .log_components();
        });
}

#[derive(Component)]
pub struct Crosshair;

fn update_cursor_pos(pos: Res<CrosshairOffset>, mut query: Query<&mut Style, With<Crosshair>>) {
    if let Ok(mut crosshair) = query.get_single_mut() {
        crosshair.position.left = Val::Px(pos.x);
        crosshair.position.bottom = Val::Px(pos.y);
    }
}

#[derive(Default, Debug, Resource)]
pub struct CrosshairOffset {
    pub x: f32,
    pub y: f32,
}

pub fn register(app: &mut App) {
    app.insert_resource(CrosshairOffset::default())
        .add_system_set(SystemSet::on_enter(GameState::Playing).with_system(add_crosshair))
        .add_system_set(SystemSet::on_update(GameState::Playing).with_system(update_cursor_pos));
}
