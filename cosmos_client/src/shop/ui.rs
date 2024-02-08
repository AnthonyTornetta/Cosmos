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
        widget::Label,
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

use crate::ui::components::{
    button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonEventType, ButtonStyles, ButtonUiSystemSet},
    scollable_container::ScrollBundle,
    slider::{Slider, SliderBundle, SliderStyles},
    text_input::{InputType, TextInput, TextInputBundle, TextInputUiSystemSet},
};

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
                    background_color: Color::DARK_GRAY.into(),
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(40.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                text_input: TextInput {
                    style: TextStyle {
                        font_size: 32.0,
                        ..Default::default()
                    },
                    input_type: InputType::Integer { min: -10000, max: 10000 },
                    ..Default::default()
                },
                ..Default::default()
            });

            p.spawn(ButtonBundle::<ClickBtnEvent> {
                node_bundle: NodeBundle {
                    style: Style {
                        width: Val::Px(400.0),
                        height: Val::Px(200.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                button: Button::<ClickBtnEvent> {
                    starting_text: Some(("Hello!".into(), text_style.clone())),
                    button_styles: Some(ButtonStyles {
                        background_color: Color::RED,
                        foreground_color: Color::BLACK,
                        hover_background_color: Color::GREEN,
                        hover_foreground_color: Color::WHITE,
                        press_background_color: Color::PURPLE,
                        press_foreground_color: Color::YELLOW,
                    }),
                    ..Default::default()
                },

                ..Default::default()
            });

            p.spawn(SliderBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        width: Val::Px(400.0),
                        height: Val::Px(200.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                slider: Slider {
                    range: 0..1001,
                    slider_styles: Some(SliderStyles {
                        hover_background_color: Color::GREEN,
                        hover_foreground_color: Color::WHITE,
                        press_background_color: Color::PURPLE,
                        press_foreground_color: Color::YELLOW,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });

            p.spawn(ScrollBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        width: Val::Px(400.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|p| {
                for i in 0..100 {
                    p.spawn((
                        TextBundle::from_section(
                            format!("Item {i}"),
                            TextStyle {
                                font: asset_server.load("fonts/PixeloidSans.ttf"),
                                font_size: 20.,
                                ..Default::default()
                            },
                        ),
                        Label,
                    ));
                }
            });
        });
}

#[derive(Event)]
struct ClickBtnEvent {}

impl ButtonEvent for ClickBtnEvent {
    fn create_event(_: ButtonEventType) -> Option<Self> {
        Some(Self {})
    }
}

fn reader(mut ev_reader: EventReader<ClickBtnEvent>) {
    for _ in ev_reader.read() {
        println!("Click event!");
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<ClickBtnEvent>(app);

    app.add_mut_event::<OpenShopUiEvent>()
        .add_systems(
            Update,
            (open_shop_ui, render_shop_ui)
                .after(NetworkingSystemsSet::FlushReceiveMessages)
                .before(TextInputUiSystemSet::ApplyDeferredA),
        )
        .add_systems(Update, reader.after(ButtonUiSystemSet::SendButtonEvents));
}
