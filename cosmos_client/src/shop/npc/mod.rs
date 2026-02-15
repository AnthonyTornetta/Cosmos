use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    faction::{FactionId, Factions},
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    npc::{
        Npc,
        shop::{ChatWithShopNpcMessage, ShopNpc, ShopNpcDialogOptions},
    },
    state::in_gameplay_state,
};

use crate::{
    entities::player::PersonMesh,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::LookingAt,
    ui::{OpenMenu, components::show_cursor::ShowCursor, font::DefaultFont},
};

fn on_add_npc(
    mut commands: Commands,
    q_shop_npc: Query<Entity, Added<ShopNpc>>,
    person_mesh: Res<PersonMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for ent in q_shop_npc.iter() {
        commands.entity(ent).insert((
            MeshMaterial3d(materials.add(StandardMaterial {
                // Makes the local player's body effectively invisible without disabling their shadow (this is stupid)
                base_color: css::PURPLE.into(),
                ..Default::default()
            })),
            Mesh3d(person_mesh.get()),
        ));
    }
}

fn on_interact_with_npc(
    q_looking_at: Query<&LookingAt, With<LocalPlayer>>,
    inputs: InputChecker,
    q_shop_npc: Query<Entity, With<ShopNpc>>,
    mut nevw_chat_npc: NettyMessageWriter<ChatWithShopNpcMessage>,
) {
    if !inputs.check_just_pressed(CosmosInputs::Interact) {
        return;
    }

    let Some(looking_at) = q_looking_at.single().ok().and_then(|x| x.looking_at_entity) else {
        return;
    };

    if !q_shop_npc.contains(looking_at) {
        return;
    }

    nevw_chat_npc.write(ChatWithShopNpcMessage { npc: looking_at });
}

fn on_recv_chat_msg(
    mut nmr_chat_to_npc: MessageReader<ShopNpcDialogOptions>,
    q_npc: Query<(&Npc, Option<&FactionId>)>,
    mut commands: Commands,
    factions: Res<Factions>,
    font: Res<DefaultFont>,
) {
    let Some(m) = nmr_chat_to_npc.read().next() else {
        return;
    };
    let Ok((npc, fac_id)) = q_npc.get(m.entity) else {
        return;
    };

    let fac = fac_id.and_then(|id| factions.from_id(id));

    commands
        .spawn((
            BackgroundColor(Srgba::hex("444444").unwrap().into()),
            Node {
                bottom: Val::Px(0.0),
                margin: UiRect::horizontal(Val::Auto),
                width: Val::Percent(80.0),
                height: Val::Px(500.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            OpenMenu::new(0),
            ShowCursor,
        ))
        .with_children(|p| {
            // name + faction
            p.spawn((Name::new("Name + Faction"), Node { ..Default::default() }))
                .with_children(|p| {
                    p.spawn((
                        TextFont {
                            font: font.get(),
                            font_size: 20.0,

                            ..Default::default()
                        },
                        Text::new(format!("{} {}", npc.first_name, npc.last_name)),
                    ))
                    .with_children(|p| {
                        if let Some(fac) = fac {
                            p.spawn((
                                Text::new(format!(" <{}>", fac.name())),
                                TextFont {
                                    font: font.get(),
                                    font_size: 20.0,

                                    ..Default::default()
                                },
                            ));
                        }
                    });
                });

            // text
            p.spawn((Name::new("Text"), Node { ..Default::default() })).with_children(|p| {
                p.spawn((
                    TextFont {
                        font: font.get(),
                        font_size: 20.0,

                        ..Default::default()
                    },
                    Text::new(&m.text),
                ));
            });

            // options
            p.spawn((
                Name::new("Options"),
                Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    TextFont {
                        font: font.get(),
                        font_size: 20.0,

                        ..Default::default()
                    },
                    Text::new("View Bounties"),
                ));
                p.spawn((
                    TextFont {
                        font: font.get(),
                        font_size: 20.0,

                        ..Default::default()
                    },
                    Text::new("Buy"),
                ));
            });
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_add_npc, on_interact_with_npc, on_recv_chat_msg)
            .run_if(in_gameplay_state)
            .chain(),
    );
}
