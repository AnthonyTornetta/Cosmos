use bevy::prelude::*;
use cosmos_core::{economy::Credits, netty::client::LocalPlayer, state::GameState};

use super::reactivity::{BindValue, BindValues, ReactableFields};

mod looking_at_tooltips;
pub mod tooltip;

fn create_credits_node(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    local_player: Query<(Entity, &Credits), (Added<Credits>, With<LocalPlayer>)>,
) {
    let Ok((local_player, credits)) = local_player.single() else {
        return;
    };

    let font = asset_server.load("fonts/PixeloidSans.ttf");

    let text_style = TextFont {
        font_size: 24.0,
        font: font.clone(),
        ..Default::default()
    };

    commands
        .spawn((
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
        .with_children(|p| {
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
    tooltip::register(app);
    looking_at_tooltips::register(app);

    app.add_systems(OnEnter(GameState::Playing), create_credits_node)
        .add_systems(Update, create_credits_node.run_if(in_state(GameState::Playing)));
}
