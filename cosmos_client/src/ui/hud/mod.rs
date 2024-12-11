use bevy::{
    app::App,
    asset::AssetServer,
    core::Name,
    ecs::{
        entity::Entity,
        query::With,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::error,
    prelude::{ChildBuild, Text},
    state::state::OnEnter,
    text::{TextFont, TextSpan},
    ui::{AlignContent, JustifyContent, Node, PositionType, TargetCamera, UiRect, Val},
};
use cosmos_core::{economy::Credits, netty::client::LocalPlayer, state::GameState};

use super::{
    reactivity::{BindValue, BindValues, ReactableFields},
    UiRoot,
};

fn create_credits_node(
    mut commands: Commands,
    q_ui_root: Query<Entity, With<UiRoot>>,
    asset_server: Res<AssetServer>,
    local_player: Query<(Entity, &Credits), With<LocalPlayer>>,
) {
    let Ok((local_player, credits)) = local_player.get_single() else {
        error!("Cannot display credits - local player entity missing!");
        return;
    };

    let font = asset_server.load("fonts/PixeloidSans.ttf");

    let text_style = TextFont {
        font_size: 24.0,
        font: font.clone(),
        ..Default::default()
    };

    let ui_root = q_ui_root.single();

    commands
        .spawn((
            TargetCamera(ui_root),
            Name::new("Credits display"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::End,
                align_content: AlignContent::Start,
                padding: UiRect::all(Val::Px(10.0)),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
        ))
        .with_children(|p: &mut bevy::prelude::ChildBuilder<'_>| {
            p.spawn((
                Name::new("Credits Text"),
                BindValues::<Credits>::new(vec![BindValue::new(local_player, ReactableFields::Text { section: 1 })]),
                text_style.clone(),
                Text::new("$"),
            ))
            .with_children(|p| {
                p.spawn((TextSpan::new(format!("{}", credits.amount())), text_style.clone()));
            });
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), create_credits_node);
}
