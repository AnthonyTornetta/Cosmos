use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css,
    core_pipeline::oit::OrderIndependentTransparencySettings,
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::Hdr,
    },
};
use cosmos_core::{
    ecs::compute_totally_accurate_global_transform,
    netty::client::LocalPlayer,
    prelude::{Asteroid, Ship, Station, Structure},
    state::GameState,
    structure::chunk::CHUNK_DIMENSIONSF,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    skybox::NeedsSkybox,
};

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
        Hdr,
        Camera {
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
        Bloom::default(),
        Camera3d::default(),
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
            BorderColor::all(css::AQUA),
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
                image: handle.0.clone(),
                ..Default::default()
            },));
        });
}

// pub struct FocusCamDistance(f32);

const TOGGLED_REASON: &str = "cosmos:toggled_off";

fn render_on_focus(
    hidden: Option<Res<FocusUiHidden>>,
    mut q_cam: Query<(Entity, &mut Transform, &mut Camera, Option<&ChildOf>), With<FocusedCam>>,
    mut focused_ui: Query<&mut HiddenReasons, With<FocusedUi>>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    // TODO: Replace this With<Structure> check w/ a FocusCamDistance component read
    q_focused: Query<&Indicating, With<FocusedWaypointEntity>>,
    q_is_valid_focus_target: Query<(), Or<(With<Asteroid>, With<Ship>, With<Station>)>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    q_trans: Query<(&Transform, Option<&ChildOf>), Without<FocusedCam>>,
) {
    let Ok((cam_entity, mut cam_trans, mut cam, parent)) = q_cam.single_mut() else {
        return;
    };

    let Ok(mut focused_reasons) = focused_ui.single_mut() else {
        if parent.is_some() {
            commands.entity(cam_entity).remove::<ChildOf>();
        }
        return;
    };

    let Some((focused_g_trans, focused_ent)) = q_focused
        .single()
        .ok()
        .filter(|indicating| q_is_valid_focus_target.contains(indicating.0))
        .and_then(|indicating| compute_totally_accurate_global_transform(indicating.0, &q_trans).map(|x| (x, indicating.0)))
    else {
        if parent.is_some() {
            commands.entity(cam_entity).remove::<ChildOf>();
        }
        cam.is_active = false;
        focused_reasons.add_reason(VIS_REASON);
        return;
    };

    let Ok(local_ent) = q_local_player.single() else {
        return;
    };

    let Some(player_g_trans) = compute_totally_accurate_global_transform(local_ent, &q_trans) else {
        return;
    };

    cam.is_active = true;
    focused_reasons.remove_reason(VIS_REASON);

    if hidden.is_some() {
        focused_reasons.add_reason(TOGGLED_REASON);
    }

    let local_player_delta = focused_g_trans.rotation().inverse() * (player_g_trans.translation() - focused_g_trans.translation());

    if parent.map(|x| x.parent()) != Some(focused_ent) {
        commands.entity(cam_entity).insert(ChildOf(focused_ent));
    }

    let cam_dist = if let Ok(structure) = q_structure.get(focused_ent) {
        let Some(neg_most) = structure.all_chunks_iter(false).next() else {
            return;
        };

        let neg_most = neg_most.coordinate();
        let mut neg_most = Vec3::new(neg_most.x as f32, neg_most.y as f32, neg_most.z as f32) * CHUNK_DIMENSIONSF;
        let mut pos_most = neg_most;

        for c in structure.all_chunks_iter(false).skip(1) {
            let coord = c.coordinate();
            let coord = Vec3::new(coord.x as f32, coord.y as f32, coord.z as f32) * CHUNK_DIMENSIONSF;
            neg_most = neg_most.min(coord);
            pos_most = pos_most.max(coord);
        }

        CHUNK_DIMENSIONSF.max((pos_most - neg_most).abs().max_element())
    } else {
        100.0
    };

    cam_trans.translation = local_player_delta.normalize_or(Vec3::Z) * cam_dist;
    cam_trans.look_at(Vec3::ZERO, focused_g_trans.rotation().inverse() * player_g_trans.up());
}

#[derive(Resource)]
struct FocusUiHidden;

fn toggle_view(mut commands: Commands, inputs: InputChecker, mut q_reasons: Query<&mut HiddenReasons, With<FocusedUi>>) {
    if inputs.check_just_pressed(CosmosInputs::ToggleFocusCam) {
        for mut r in q_reasons.iter_mut() {
            if !r.remove_reason(TOGGLED_REASON) {
                commands.insert_resource(FocusUiHidden);
                r.add_reason(TOGGLED_REASON);
            } else {
                commands.remove_resource::<FocusUiHidden>();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), (setup_camera, create_focused_ui).chain())
        .add_systems(Update, (render_on_focus, toggle_view));
}
