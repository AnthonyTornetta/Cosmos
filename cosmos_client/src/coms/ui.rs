use bevy::{color::palettes::css, prelude::*};
use cosmos_core::state::GameState;

use crate::{
    asset::asset_loader::load_assets,
    ui::{
        components::button::{register_button, ButtonEvent, CosmosButton},
        font::DefaultFont,
    },
};

fn create_coms_ui(mut commands: Commands, coms_assets: Res<ComsAssets>, font: Res<DefaultFont>) {
    let accent: Color = css::AQUA.into();
    let main: Color = Srgba::hex("#555").unwrap().into();
    let main_transparent: Color = Srgba::hex("#555555DE").unwrap().into();
    let border: Color = Srgba::hex("#222").unwrap().into();

    let font = TextFont {
        font: font.0.clone_weak(),
        font_size: 24.0,
        ..Default::default()
    };

    commands
        .spawn((
            Name::new("Coms Ui"),
            Node {
                margin: UiRect::new(Val::Auto, Val::Px(0.0), Val::Auto, Val::Px(0.0)),
                height: Val::Percent(85.0),
                width: Val::Px(450.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Coms Header"),
                Node {
                    margin: UiRect::left(Val::Px(50.0)),
                    height: Val::Px(40.0),
                    flex_direction: FlexDirection::Row,
                    max_width: Val::Px(400.0),
                    ..Default::default()
                },
                BorderRadius {
                    top_left: Val::Px(5.0),
                    ..Default::default()
                },
                BackgroundColor(Srgba::hex("#232323").unwrap().into()),
            ))
            .with_children(|p| {
                let btn_node = Node {
                    width: Val::Px(30.0),
                    ..Default::default()
                };

                p.spawn((
                    Name::new("Left btn"),
                    BorderRadius {
                        top_left: Val::Px(5.0),
                        ..Default::default()
                    },
                    BackgroundColor(accent),
                    CosmosButton::<LeftClicked> {
                        text: Some(("<".into(), font.clone(), Default::default())),
                        ..Default::default()
                    },
                    btn_node.clone(),
                ));

                p.spawn((
                    Text::new("Cool Ship"),
                    font.clone(),
                    TextLayout {
                        justify: JustifyText::Center,
                        ..Default::default()
                    },
                    Node {
                        flex_grow: 1.0,
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Name::new("Right btn"),
                    BackgroundColor(accent),
                    CosmosButton::<RightClicked> {
                        text: Some((">".into(), font.clone(), Default::default())),
                        ..Default::default()
                    },
                    btn_node,
                ));
            });

            p.spawn((
                Name::new("Main Content"),
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Expand Button"),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(100.0),
                        ..Default::default()
                    },
                    BorderRadius {
                        top_left: Val::Px(5.0),
                        bottom_left: Val::Px(5.0),
                        ..Default::default()
                    },
                    ImageNode::new(coms_assets.close.clone_weak()),
                    BackgroundColor(accent),
                ));
                p.spawn((
                    Name::new("Body"),
                    Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    BackgroundColor(main_transparent),
                    BorderColor(accent),
                ));
            });
        });
}

#[derive(Resource, Debug)]
pub struct ComsAssets {
    open: Handle<Image>,
    close: Handle<Image>,
}

#[derive(Event, Debug)]
struct LeftClicked;

impl ButtonEvent for LeftClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}
#[derive(Event, Debug)]
struct RightClicked;

impl ButtonEvent for RightClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), create_coms_ui);

    register_button::<LeftClicked>(app);
    register_button::<RightClicked>(app);

    load_assets::<Image, ComsAssets, 2>(
        app,
        GameState::Loading,
        ["cosmos/images/ui/open-coms.png", "cosmos/images/ui/close-coms.png"],
        |mut cmds, [(open, _), (close, _)]| cmds.insert_resource(ComsAssets { open, close }),
    );
}
