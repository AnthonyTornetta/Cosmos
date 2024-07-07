use bevy::{
    app::App,
    asset::AssetServer,
    color::palettes::css,
    core::Name,
    ecs::{
        entity::Entity,
        query::With,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::error,
    state::state::OnEnter,
    text::{Text, TextSection, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        AlignContent, JustifyContent, PositionType, Style, UiRect, Val,
    },
};
use cosmos_core::{economy::Credits, netty::client::LocalPlayer};

use crate::state::game_state::GameState;

use super::reactivity::{BindValue, BindValues, ReactableFields};

fn create_credits_node(mut commands: Commands, asset_server: Res<AssetServer>, local_player: Query<(Entity, &Credits), With<LocalPlayer>>) {
    let Ok((local_player, credits)) = local_player.get_single() else {
        error!("Cannot display credits - local player entity missing!");
        return;
    };

    let font = asset_server.load("fonts/PixeloidSans.ttf");

    let text_style = TextStyle {
        color: css::WHITE,
        font_size: 24.0,
        font: font.clone(),
    };

    commands
        .spawn((
            Name::new("Credits display"),
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::End,
                    align_content: AlignContent::Start,
                    padding: UiRect::all(Val::Px(10.0)),
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|p: &mut bevy::prelude::ChildBuilder<'_>| {
            p.spawn((
                Name::new("Credits Text"),
                BindValues::<Credits>::new(vec![BindValue::new(local_player, ReactableFields::Text { section: 1 })]),
                TextBundle {
                    text: Text::from_sections([
                        TextSection::new("$", text_style.clone()),
                        TextSection::new(format!("{}", credits.amount()), text_style.clone()),
                    ]),
                    ..Default::default()
                },
            ));
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), create_credits_node);
}
