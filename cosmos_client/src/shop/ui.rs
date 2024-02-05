use bevy::{
    app::{App, Update},
    asset::AssetServer,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, With},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    render::color::Color,
    text::{Text, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        Style, UiRect, Val,
    },
};
use cosmos_core::{
    ecs::{
        mut_events::{MutEvent, MutEventsCommand},
        NeedsDespawned,
    },
    netty::system_sets::NetworkingSystemsSet,
    shop::Shop,
};

use crate::ui::components::text_input::{TextInput, TextInputBundle, TextInputUiSystemSet};

#[derive(Event)]
pub(super) struct OpenShopUiEvent(pub Shop);

#[derive(Component)]
struct ShopUI {
    shop: Shop,
}

fn open_shop_ui(mut commands: Commands, mut ev_reader: EventReader<MutEvent<OpenShopUiEvent>>, q_open_shops: Query<Entity, With<ShopUI>>) {
    for ev in ev_reader.read() {
        let shop = std::mem::take(&mut ev.write().0);

        println!("Display: {shop:?}");

        for ent in q_open_shops.iter() {
            commands.entity(ent).insert(NeedsDespawned);
        }

        commands.spawn(ShopUI { shop });
    }
}

fn render_shop_ui(mut commands: Commands, q_shop_ui: Query<(&ShopUI, Entity), Added<ShopUI>>, asset_server: Res<AssetServer>) {
    let Ok((shop_ui, ui_ent)) = q_shop_ui.get_single() else {
        return;
    };

    let name = &shop_ui.shop.name;

    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    commands
        .entity(ui_ent)
        .insert(NodeBundle {
            background_color: Color::BLACK.into(),
            style: Style {
                width: Val::Px(800.0),
                height: Val::Px(800.0),
                left: Val::Percent(51.0),
                margin: UiRect {
                    // Centers it vertically
                    top: Val::Auto,
                    bottom: Val::Auto,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn(TextBundle {
                text: Text::from_section(name, text_style.clone()),
                ..Default::default()
            });

            p.spawn(TextInputBundle {
                node_bundle: NodeBundle {
                    background_color: Color::GRAY.into(),
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(40.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                text_input: TextInput::new(TextStyle {
                    font_size: 32.0,
                    ..Default::default()
                }),
                ..Default::default()
            });
        });
}

pub(super) fn register(app: &mut App) {
    app.add_mut_event::<OpenShopUiEvent>().add_systems(
        Update,
        (open_shop_ui, render_shop_ui)
            .after(NetworkingSystemsSet::FlushReceiveMessages)
            .before(TextInputUiSystemSet::ApplyDeferredA),
    );
}
