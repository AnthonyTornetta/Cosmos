use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css,
    core_pipeline::{bloom::Bloom, oit::OrderIndependentTransparencySettings},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use cosmos_core::{netty::client::LocalPlayer, prelude::Structure, state::GameState};

use crate::skybox::NeedsSkybox;

use super::{
    hide::HiddenReasons,
    ship_flight::indicators::{FocusedWaypointEntity, Indicating},
};

#[derive(Component)]
struct FocusedCam;

fn setup_rendered_item_atlas(mut images: ResMut<Assets<Image>>, w: u32, h: u32) -> Handle<Image> {
    let size = Extent3d {
        width: w,
        height: h,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // You need to set these texture usage flags in order to use the image as a render target
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    images.add(image)
}

#[derive(Resource, Debug)]
struct FocusCamImage(Handle<Image>);

fn setup_camera(mut commands: Commands, images: ResMut<Assets<Image>>) {
    let width = 512;
    let height = 512;

    let image_handle = setup_rendered_item_atlas(images, width, height);

    commands.insert_resource(FocusCamImage(image_handle.clone()));

    commands.spawn((
        Camera {
            hdr: true,
            target: image_handle.clone().into(),
            is_active: false,
            ..Default::default()
        },
        NeedsSkybox,
        Transform::from_translation(Vec3::ZERO),
        Projection::from(PerspectiveProjection {
            fov: (90.0 / 180.0) * std::f32::consts::PI,
            ..default()
        }),
        Camera3d::default(),
        Bloom { ..Default::default() },
        Name::new("Focused Camera"),
        OrderIndependentTransparencySettings::default(),
        FocusedCam,
        Msaa::Off,
    ));
}

#[derive(Component)]
struct FocusedUi;

const VIS_REASON: &str = "cosmos:ship_not_focused";

fn create_focused_ui(mut commands: Commands, handle: Res<FocusCamImage>) {
    let mut hidden = HiddenReasons::default();
    hidden.add_reason(VIS_REASON);

    commands
        .spawn((
            Name::new("Focused Camera Display"),
            BorderColor(css::AQUA.into()),
            FocusedUi,
            hidden,
            Visibility::Hidden,
            Node {
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                position_type: PositionType::Absolute,
                width: Val::Px(512.0),
                height: Val::Px(512.0),
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((ImageNode {
                image: handle.0.clone_weak(),
                ..Default::default()
            },));
        });
}

// pub struct FocusCamDistance(f32);

fn render_on_focus(
    mut q_cam: Query<(&mut Transform, &mut Camera), With<FocusedCam>>,
    mut focused_ui: Query<&mut HiddenReasons, With<FocusedUi>>,
    q_local_player_trans: Query<&GlobalTransform, With<LocalPlayer>>,
    // TODO: Replace this With<Structure> check w/ a FocusCamDistance component read
    q_g_trans: Query<&GlobalTransform, With<Structure>>,
    q_focused: Query<&Indicating, With<FocusedWaypointEntity>>,
) {
    let Ok((mut cam_trans, mut cam)) = q_cam.get_single_mut() else {
        return;
    };

    let Ok(mut focused_reasons) = focused_ui.get_single_mut() else {
        return;
    };

    let Some(focused_g_trans) = q_focused.get_single().ok().and_then(|indicating| q_g_trans.get(indicating.0).ok()) else {
        cam.is_active = false;
        focused_reasons.add_reason(VIS_REASON);
        return;
    };

    let Ok(player_g_trans) = q_local_player_trans.get_single() else {
        return;
    };

    cam.is_active = true;
    focused_reasons.remove_reason(VIS_REASON);

    let player_delta = player_g_trans.translation() - focused_g_trans.translation();

    cam_trans.translation = player_delta.normalize_or_zero() * 100.0 + focused_g_trans.translation();
    cam_trans.look_at(focused_g_trans.translation(), player_g_trans.up());
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), (setup_camera, create_focused_ui).chain())
        .add_systems(Update, render_on_focus);
}
