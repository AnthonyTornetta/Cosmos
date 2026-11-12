use bevy::prelude::*;
use cosmos_core::{
    coms::events::{AcceptComsMessage, DeclineComsMessage},
    netty::sync::events::client_event::NettyMessageWriter,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::font::DefaultFont,
};

use super::style;

#[derive(Message, Debug)]
pub(crate) struct OpenRequestComsUi(pub Entity);

#[derive(Component)]
struct RenderedComsRequestUi(Entity);

fn on_open_req_coms_ui(
    mut commands: Commands,
    mut evr_open_request_ui: MessageReader<OpenRequestComsUi>,
    font: Res<DefaultFont>,
    q_names: Query<&Name>,
    q_requests: Query<Entity, With<RenderedComsRequestUi>>,
    inputs: InputChecker,
) {
    if let Some(ev) = evr_open_request_ui.read().last() {
        for entity in &q_requests {
            commands.entity(entity).despawn();
        }
        let ship_name = q_names.get(ev.0).map(|name| name.as_str()).unwrap_or("Unknown vessel");
        let accept = inputs
            .get_control(CosmosInputs::AcceptComsRequest)
            .map(|key| key.to_string())
            .unwrap_or("Unbound".into());
        let decline = inputs
            .get_control(CosmosInputs::DeclineComsRequest)
            .map(|key| key.to_string())
            .unwrap_or("Unbound".into());

        commands
            .spawn((
                RenderedComsRequestUi(ev.0),
                Name::new("Coms Request UI"),
                DespawnOnExit(cosmos_core::state::GameState::Playing),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(400.0),
                    max_width: Val::Vw(96.0),
                    right: Val::Px(12.0),
                    top: Val::Percent(18.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    row_gap: Val::Px(10.0),
                    border: UiRect::top(Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                BackgroundColor(style::BACKGROUND),
                BorderColor::all(style::ACCENT),
            ))
            .with_children(|p| {
                style::label(p, &font.0, "INCOMING HAIL", 12.0, style::ACCENT);
                style::label(p, &font.0, ship_name, 22.0, style::TEXT);
                style::label(
                    p,
                    &font.0,
                    &format!("{accept} - Accept     {decline} - Decline"),
                    13.0,
                    style::MUTED,
                );
            });
    }
}

fn press_accept(
    mut nevw_accept_coms: NettyMessageWriter<AcceptComsMessage>,
    mut commands: Commands,
    inputs: InputChecker,
    q_rendered_coms_req_ui: Query<(Entity, &RenderedComsRequestUi)>,
) {
    if !inputs.check_just_pressed(CosmosInputs::AcceptComsRequest) {
        return;
    }
    let Ok((ent, rendered_req_coms_ui)) = q_rendered_coms_req_ui.single() else {
        return;
    };

    commands.entity(ent).despawn();
    nevw_accept_coms.write(AcceptComsMessage(rendered_req_coms_ui.0));
}

fn press_decline(
    mut nevw_decline_coms: NettyMessageWriter<DeclineComsMessage>,
    mut commands: Commands,
    inputs: InputChecker,
    q_rendered_coms_req_ui: Query<Entity, With<RenderedComsRequestUi>>,
) {
    if !inputs.check_just_pressed(CosmosInputs::DeclineComsRequest) {
        return;
    }
    let Ok(ent) = q_rendered_coms_req_ui.single() else {
        return;
    };

    nevw_decline_coms.write_default();

    commands.entity(ent).despawn();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (on_open_req_coms_ui, press_decline, press_accept).chain())
        .add_message::<OpenRequestComsUi>();
}
