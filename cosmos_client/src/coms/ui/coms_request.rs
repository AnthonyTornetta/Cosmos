use bevy::prelude::*;
use cosmos_core::{
    coms::events::{AcceptComsEvent, DeclineComsEvent},
    netty::{sync::events::client_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::font::DefaultFont,
};

#[derive(Event, Debug)]
pub(crate) struct OpenRequestComsUi(pub Entity);

#[derive(Component)]
struct RenderedComsRequestUi(Entity);

fn on_open_req_coms_ui(mut commands: Commands, mut evr_open_request_ui: EventReader<OpenRequestComsUi>, font: Res<DefaultFont>) {
    for ev in evr_open_request_ui.read() {
        let title_font = TextFont {
            font: font.0.clone(),
            font_size: 24.0,
            ..Default::default()
        };

        let text_font = TextFont {
            font: font.0.clone(),
            font_size: 20.0,
            ..Default::default()
        };

        commands
            .spawn((
                RenderedComsRequestUi(ev.0),
                Name::new("Coms Request UI"),
                Node {
                    height: Val::Px(100.0),
                    width: Val::Px(400.0),
                    right: Val::Px(0.0),
                    top: Val::Percent(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                BackgroundColor(Srgba::hex("#333333EE").unwrap().into()),
            ))
            .with_children(|p| {
                p.spawn((Text::new("[Ship Name] is Hailing You"), title_font.clone()));

                p.spawn((Text::new("Y - Accept | N - Decline"), text_font.clone()));
            });
    }
}

fn press_accept(
    mut nevw_accept_coms: NettyEventWriter<AcceptComsEvent>,
    mut commands: Commands,
    inputs: InputChecker,
    q_rendered_coms_req_ui: Query<(Entity, &RenderedComsRequestUi)>,
) {
    if !inputs.check_just_pressed(CosmosInputs::AcceptComsRequest) {
        return;
    }
    let Ok((ent, rendered_req_coms_ui)) = q_rendered_coms_req_ui.get_single() else {
        return;
    };

    commands.entity(ent).despawn_recursive();
    info!("Sending ACC");
    nevw_accept_coms.send(AcceptComsEvent(rendered_req_coms_ui.0));
}

fn press_decline(
    mut nevw_decline_coms: NettyEventWriter<DeclineComsEvent>,
    mut commands: Commands,
    inputs: InputChecker,
    q_rendered_coms_req_ui: Query<Entity, With<RenderedComsRequestUi>>,
) {
    if !inputs.check_just_pressed(CosmosInputs::DeclineComsRequest) {
        return;
    }
    let Ok(ent) = q_rendered_coms_req_ui.get_single() else {
        return;
    };

    nevw_decline_coms.send_default();

    commands.entity(ent).despawn_recursive();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_open_req_coms_ui, press_decline, press_accept)
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    )
    .add_event::<OpenRequestComsUi>();
}
