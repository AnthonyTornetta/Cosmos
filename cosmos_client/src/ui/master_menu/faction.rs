use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    faction::{
        Faction, FactionId, Factions,
        events::{PlayerCreateFactionEvent, PlayerCreateFactionEventResponse, PlayerLeaveFactionEvent},
    },
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    state::GameState,
};

use crate::{
    create_button_event,
    ui::{
        components::{
            button::{CosmosButton, register_button},
            modal::{
                Modal,
                confirm_modal::{ConfirmModal, ConfirmModalComplete},
                text_modal::{TextModal, TextModalComplete},
            },
            text_input::InputType,
        },
        font::DefaultFont,
        hud::error::ShowInfoPopup,
    },
};

#[derive(Component)]
#[require(Node)]
pub struct FactionDisplay;

create_button_event!(CreateFaction);
create_button_event!(LeaveFaction);
create_button_event!(InviteToFaction);

fn render_with_faction(p: &mut RelatedSpawnerCommands<ChildOf>, faction: &Faction, font: &DefaultFont) {
    p.spawn(
        (Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::all(Val::Px(20.0)),
            ..Default::default()
        }),
    )
    .with_children(|p| {
        p.spawn((
            Text::new(faction.name()),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..Default::default()
            },
            TextFont {
                font_size: 32.0,
                font: font.get(),
                ..Default::default()
            },
            TextColor(css::AQUA.into()),
        ));

        p.spawn(
            (Node {
                flex_grow: 1.0,
                ..Default::default()
            }),
        )
        .with_children(|p| {
            p.spawn(
                (Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    margin: UiRect::right(Val::Px(10.0)),
                    ..Default::default()
                }),
            )
            .with_children(|p| {
                p.spawn((
                    BackgroundColor(css::AQUA.into()),
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..Default::default()
                    },
                    CosmosButton::<InviteToFaction> {
                        text: Some((
                            "Invite to Faction".into(),
                            TextFont {
                                font_size: 24.0,
                                font: font.get(),
                                ..Default::default()
                            },
                            TextColor(css::BLACK.into()),
                        )),
                        ..Default::default()
                    },
                ));

                p.spawn((
                    BackgroundColor(css::DARK_RED.into()),
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..Default::default()
                    },
                    CosmosButton::<LeaveFaction> {
                        text: Some((
                            "Leave Faction".into(),
                            TextFont {
                                font_size: 24.0,
                                font: font.get(),
                                ..Default::default()
                            },
                            Default::default(),
                        )),
                        ..Default::default()
                    },
                ));
            });

            p.spawn(
                (Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    margin: UiRect::left(Val::Px(10.0)),
                    ..Default::default()
                }),
            )
            .with_children(|p| {
                p.spawn((
                    Text::new("Members"),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    Node {
                        margin: UiRect::bottom(Val::Px(20.0)),
                        ..Default::default()
                    },
                ));

                for player in faction.players() {
                    p.spawn((
                        Text::new(format!("{player:?}")),
                        TextFont {
                            font_size: 16.0,
                            font: font.get(),
                            ..Default::default()
                        },
                        Node {
                            margin: UiRect::bottom(Val::Px(20.0)),
                            ..Default::default()
                        },
                    ));
                }
            });
        });
    });
}

fn render_no_faction(p: &mut RelatedSpawnerCommands<ChildOf>, font: &DefaultFont) {
    p.spawn(
        (Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::all(Val::Px(50.0)),
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
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..Default::default()
            },
        ));

        p.spawn((
            BackgroundColor(css::AQUA.into()),
            Node {
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(10.0)),
                ..Default::default()
            },
            CosmosButton::<CreateFaction> {
                text: Some((
                    "Create Faction".into(),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    TextColor(css::BLACK.into()),
                )),
                ..Default::default()
            },
        ));
    });
}

#[derive(Component)]
struct RerenderFactionDisplay;

fn render_faction_display(
    mut commands: Commands,
    q_added_fac_display: Query<Entity, Or<(Added<RerenderFactionDisplay>, Added<FactionDisplay>)>>,
    q_faction: Query<&FactionId, With<LocalPlayer>>,
    font: Res<DefaultFont>,
    factions: Res<Factions>,
) {
    for ent in q_added_fac_display.iter() {
        commands
            .entity(ent)
            .remove::<RerenderFactionDisplay>()
            .despawn_related::<Children>()
            .insert((Name::new("Faction Display"),))
            .with_children(|p| {
                if let Ok(fac_id) = q_faction.single() {
                    let Some(fac) = factions.from_id(fac_id) else {
                        render_no_faction(p, &font);
                        error!("Missing faction for faction id {fac_id:?}!");
                        return;
                    };

                    render_with_faction(p, fac, &font);
                } else {
                    render_no_faction(p, &font);
                }
            });
    }
}

#[derive(Component)]
struct FactionNameBox;

fn on_create_faction_click(
    mut evr_create_faction: EventReader<CreateFaction>,
    q_faction_box: Query<Entity, With<FactionNameBox>>,
    mut commands: Commands,
) {
    if !evr_create_faction.read().next().is_some() {
        return;
    }

    if q_faction_box.iter().next().is_some() {
        return;
    }

    commands
        .spawn((
            FactionNameBox,
            Name::new("Faction Name Box"),
            Modal {
                title: "Faction Name".into(),
            },
            TextModal {
                input_type: InputType::Text { max_length: Some(30) },
                ..Default::default()
            },
        ))
        .observe(
            |ev: Trigger<TextModalComplete>, mut nevw_create_faction: NettyEventWriter<PlayerCreateFactionEvent>| {
                nevw_create_faction.write(PlayerCreateFactionEvent {
                    faction_name: ev.text.clone(),
                });

                info!("Sending create for {ev:?}");
            },
        );
}

fn get_faction_response(
    mut nevr_create_response: EventReader<PlayerCreateFactionEventResponse>,
    q_faction_name_box: Query<Entity, With<FactionNameBox>>,
    mut errors: EventWriter<ShowInfoPopup>,
    mut commands: Commands,
) {
    for ev in nevr_create_response.read() {
        match ev {
            PlayerCreateFactionEventResponse::NameTaken => {
                errors.write(ShowInfoPopup::error("Faction name already taken."));
            }
            PlayerCreateFactionEventResponse::ServerError => {
                errors.write(ShowInfoPopup::error("Something bad happened - check server logs."));
            }
            PlayerCreateFactionEventResponse::NameTooLong => {
                errors.write(ShowInfoPopup::error("Faction name too long."));
            }
            PlayerCreateFactionEventResponse::AlreadyInFaction => {
                errors.write(ShowInfoPopup::error("You cannot create a faction while being in one."));
            }
            PlayerCreateFactionEventResponse::Success => {
                let Ok(modal) = q_faction_name_box.single() else {
                    return;
                };

                commands.entity(modal).insert(NeedsDespawned);
            }
        }
    }
}

fn on_leave_faction(mut commands: Commands) {
    commands
        .spawn((
            Modal {
                title: "Leave Faction".into(),
            },
            ConfirmModal {
                prompt: "Are you sure you want to leave your faction?".into(),
                ..Default::default()
            },
        ))
        .observe(
            |ev: Trigger<ConfirmModalComplete>, mut nevw_leave_faction: NettyEventWriter<PlayerLeaveFactionEvent>| {
                if !ev.confirmed {
                    return;
                }

                nevw_leave_faction.write_default();
            },
        );
}

/*
on_create_faction,
on_leave_faction,
on_invite_player,
on_accept_invite,
* */

fn on_change_faction(
    mut removed_comps: RemovedComponents<FactionId>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_local_faction_changed: Query<(), (With<LocalPlayer>, Changed<FactionId>)>,
    q_rendered_ui: Query<Entity, With<FactionDisplay>>,
    factions: Res<Factions>,
    mut commands: Commands,
) {
    let re_render =
        // WARNING: By re-rendering on all [`Factions`] changes, we can cause issues if multiple
        // players are editing unrelated (and related) factions. I don't feel like fixing this rn,
        // so too bad.
        factions.is_changed() || !q_local_faction_changed.is_empty() || removed_comps.read().any(|x| q_local_player.contains(x));

    if re_render {
        let Ok(render_ui) = q_rendered_ui.single() else {
            return;
        };

        commands.entity(render_ui).insert(RerenderFactionDisplay);
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<CreateFaction>(app);
    register_button::<LeaveFaction>(app);
    register_button::<InviteToFaction>(app);

    app.add_systems(
        FixedUpdate,
        on_change_faction.run_if(in_state(GameState::Playing)).in_set(FixedUpdateSet::Main),
    )
    .add_systems(
        Update,
        (
            render_faction_display,
            on_create_faction_click,
            get_faction_response,
            on_leave_faction.run_if(on_event::<LeaveFaction>),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
