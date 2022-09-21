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
            parent.spawn_bundle(NodeBundle {
                image: asset_server.load("images/ui/crosshair.png").into(),
                style: Style {
                    size: Size::new(Val::Px(8.0), Val::Px(8.0)),
                    ..default()
                },

                // color: Color::NONE.into(),
                ..default()
            });
        });
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(add_crosshair));
}
