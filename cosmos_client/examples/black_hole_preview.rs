//! Render the production black hole without starting Steam, a server, or a game session.
//!
//! Run from the workspace root:
//! `cargo run -p cosmos_client --example black_hole_preview -- /tmp/black-hole-preview`
//! An optional second argument overrides the black hole radius (default: the game's 800,000).
//! Add `--animate` to also capture eight seconds of animation at 12 frames per second.

use std::{path::PathBuf, time::Duration};

use bevy::{
    camera::{Hdr, RenderTarget},
    core_pipeline::oit::OrderIndependentTransparencySettings,
    light::Skybox,
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{PollType, TextureFormat, TextureViewDescriptor, TextureViewDimension},
        renderer::RenderDevice,
        view::screenshot::{Screenshot, save_to_disk},
    },
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use cosmos_core::{physics::location::SECTOR_DIMENSIONS, universe::black_hole::BlackHole};

#[path = "../src/universe/black_hole.rs"]
mod black_hole;

fn update(app: &mut App) {
    app.update();
    app.world()
        .resource::<RenderDevice>()
        .wgpu_device()
        .poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("GPU rendering failed");
}

fn capture(app: &mut App, target: &Handle<Image>, path: PathBuf) {
    if path.exists() {
        std::fs::remove_file(&path).expect("Could not replace previous screenshot");
    }
    app.world_mut()
        .spawn(Screenshot::image(target.clone()))
        .observe(save_to_disk(path.clone()));
    for _ in 0..40 {
        update(app);
        if path.exists() {
            break;
        }
    }
    assert!(path.exists(), "Screenshot was not saved: {}", path.display());
}

fn compare_transparency_modes(output: &std::path::Path, name: &str) {
    let oit_camera = image::open(output.join(format!("{name}.png"))).unwrap().into_rgb8();
    let regular_camera = image::open(output.join(format!("{name}-regular.png"))).unwrap().into_rgb8();
    assert_eq!(oit_camera.dimensions(), regular_camera.dimensions());
    let max_difference = oit_camera
        .as_raw()
        .iter()
        .zip(regular_camera.as_raw())
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert!(
        max_difference <= 2,
        "{name}: enabling camera OIT changed the disk (maximum channel difference: {max_difference}/255)"
    );
    println!("PASS {name}: camera transparency modes agree within {max_difference}/255");
}

fn compare_lensing(app: &mut App, camera: Entity, target: &Handle<Image>, output: &std::path::Path, name: &str) {
    app.world_mut().entity_mut(camera).insert(black_hole::BlackHoleLensCamera);
    for _ in 0..8 {
        update(app);
    }
    capture(app, target, output.join(format!("{name}-on.png")));
    app.world_mut().entity_mut(camera).remove::<black_hole::BlackHoleLensCamera>();
    for _ in 0..8 {
        update(app);
    }
    capture(app, target, output.join(format!("{name}-off.png")));
    app.world_mut().entity_mut(camera).insert(black_hole::BlackHoleLensCamera);
}

fn verify_lensing(app: &mut App, camera: Entity, target: &Handle<Image>, output: &std::path::Path, radius: f32) {
    // Isolate lens geometry/depth from the glow added by the secondary image.
    app.world_mut().entity_mut(camera).remove::<Bloom>();
    let transform = Transform::from_translation(Vec3::new(0.0, 1.0, 6.0) * radius).looking_at(Vec3::ZERO, Vec3::Y);
    app.world_mut()
        .entity_mut(camera)
        .insert((transform, OrderIndependentTransparencySettings::default()));
    compare_lensing(app, camera, target, output, "lens");
    let on = image::open(output.join("lens-on.png")).unwrap().into_rgb8();
    let off = image::open(output.join("lens-off.png")).unwrap().into_rgb8();
    let changed = on
        .pixels()
        .zip(off.pixels())
        .filter(|(a, b)| a.0.iter().zip(b.0).any(|(a, b)| a.abs_diff(b) > 2))
        .count();
    assert!(changed > 100, "Lensing did not alter the rendered scene");
    // Distant pixels and the center of the shadow must remain unchanged.
    for (x, y) in [(30, 30), (1250, 770), (640, 400)] {
        assert_eq!(
            on.get_pixel(x, y),
            off.get_pixel(x, y),
            "Lensing changed a protected pixel at {x}, {y}"
        );
    }
    println!("PASS lensing: {changed} pixels bent, shadow center and distant pixels preserved");

    let arc = on.get_pixel(640, 240).0;
    let sky = off.get_pixel(640, 240).0;
    assert!(
        arc[0] > sky[0].saturating_add(40) && arc[0] > arc[2],
        "Far-side disk image did not wrap above the shadow"
    );
    println!("PASS wrapped image: far-side disk emission appears above the shadow");
    // The front disk silhouette must not undergo the old radial wobble.
    for x in [480, 560, 640, 720, 800] {
        let bottom = |image: &image::RgbImage| {
            (400..780)
                .rev()
                .find(|&y| {
                    let color = image.get_pixel(x, y).0;
                    color[0] > 120 && color[1] > 40 && color[0] > color[2]
                })
                .expect("Direct disk was not rendered")
        };
        assert!(
            bottom(&on).abs_diff(bottom(&off)) <= 2,
            "Lensing bent the direct disk silhouette at x={x}"
        );
    }
    println!("PASS direct disk: front silhouette stays fixed with lensing enabled");

    let foreground_center = transform.transform_point(Vec3::new(0.1, 0.44, -2.0) * radius);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::from_length(radius * 0.22));
    let material = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
        base_color: Color::srgb(0.02, 0.7, 0.95),
        unlit: true,
        ..default()
    });
    let foreground = app
        .world_mut()
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(foreground_center).with_rotation(transform.rotation),
        ))
        .id();
    compare_lensing(app, camera, target, output, "foreground");
    let camera_entity = app.world().entity(camera);
    let center = camera_entity
        .get::<Camera>()
        .unwrap()
        .world_to_viewport(camera_entity.get::<GlobalTransform>().unwrap(), foreground_center)
        .unwrap()
        .as_uvec2();
    let on = image::open(output.join("foreground-on.png")).unwrap().into_rgb8();
    let off = image::open(output.join("foreground-off.png")).unwrap().into_rgb8();
    let color = on.get_pixel(center.x, center.y).0;
    assert!(
        color[2] > color[0] && color[1] > color[0],
        "Foreground test object was not rendered"
    );
    for y in center.y - 8..=center.y + 8 {
        for x in center.x - 8..=center.x + 8 {
            assert_eq!(on.get_pixel(x, y), off.get_pixel(x, y), "Lensing distorted foreground geometry");
        }
    }
    println!("PASS foreground: nearby geometry remains pixel-identical with lensing on and off");
    app.world_mut().despawn(foreground);

    // Turning away must remove the effect rather than leave its last position behind.
    app.world_mut()
        .entity_mut(camera)
        .insert(Transform::from_translation(transform.translation).looking_at(transform.translation * 2.0, Vec3::Y));
    compare_lensing(app, camera, target, output, "behind-camera");
    let on = image::open(output.join("behind-camera-on.png")).unwrap().into_rgb8();
    let off = image::open(output.join("behind-camera-off.png")).unwrap().into_rgb8();
    assert!(
        on.as_raw() == off.as_raw(),
        "Lens remained active with the black hole behind the camera"
    );
    println!("PASS behind camera: lens disabled without residual distortion");

    // Camera roll and an off-center hole exercise the projected center/radius.
    let off_center = Transform::from_translation(transform.translation).looking_at(Vec3::new(2.5, 0.0, 0.0) * radius, Vec3::Y);
    app.world_mut().entity_mut(camera).insert(Transform {
        rotation: off_center.rotation * Quat::from_rotation_z(0.35),
        ..off_center
    });
    compare_lensing(app, camera, target, output, "off-center");

    app.world_mut().entity_mut(camera).insert(transform);
    app.world_mut().entity_mut(camera).get_mut::<Camera>().unwrap().viewport = Some(bevy::camera::Viewport {
        physical_position: UVec2::new(160, 100),
        physical_size: UVec2::new(960, 600),
        ..default()
    });
    compare_lensing(app, camera, target, output, "viewport");
    let on = image::open(output.join("viewport-on.png")).unwrap().into_rgb8();
    let off = image::open(output.join("viewport-off.png")).unwrap().into_rgb8();
    for (x, y) in [(30, 30), (1250, 770)] {
        assert_eq!(on.get_pixel(x, y), off.get_pixel(x, y), "Lensing escaped the camera viewport");
    }
    println!("PASS viewport: lens respects a camera viewport smaller than its render target");
    app.world_mut().entity_mut(camera).get_mut::<Camera>().unwrap().viewport = None;
    app.world_mut().entity_mut(camera).insert(Bloom::default());
    compare_lensing(app, camera, target, output, "appearance");
}

fn main() {
    let animate = std::env::args_os().any(|value| value == "--animate");
    let mut arguments = std::env::args_os().skip(1).filter(|value| value != "--animate");
    let output = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/black-hole-preview"));
    let radius = arguments
        .next()
        .map(|value| {
            value
                .to_str()
                .expect("Radius must be a number")
                .parse::<f32>()
                .expect("Radius must be a number")
        })
        .unwrap_or(SECTOR_DIMENSIONS * 40.0);
    assert!(radius.is_finite() && radius > 0.0, "Radius must be positive and finite");
    std::fs::create_dir_all(&output).expect("Could not create screenshot directory");

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>(),
    )
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    black_hole::register(&mut app);
    app.finish();
    app.cleanup();

    let target =
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new_target_texture(1280, 800, TextureFormat::Rgba8UnormSrgb, None));
    let skybox: Handle<Image> = app.world().resource::<AssetServer>().load("skybox/skybox.png");
    app.world_mut().spawn((BlackHole { radius }, Transform::IDENTITY));
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            black_hole::BlackHoleLensCamera,
            Hdr,
            RenderTarget::Image(target.clone().into()),
            Projection::from(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_3,
                ..default()
            }),
            Bloom::default(),
            OrderIndependentTransparencySettings::default(),
            Msaa::Off,
            Transform::from_translation(Vec3::new(0.0, 7.0, 10.0) * radius).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    let mut skybox_ready = false;
    for _ in 0..240 {
        update(&mut app);
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        if let Some(mut image) = images.get_mut(&skybox) {
            let layers = image.height() / image.width();
            image.reinterpret_stacked_2d_as_array(layers).expect("Invalid skybox");
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
            skybox_ready = true;
            break;
        }
    }
    assert!(skybox_ready, "Skybox did not load");
    app.world_mut().entity_mut(camera).insert(Skybox {
        image: Some(skybox),
        brightness: 1000.0,
        rotation: Quat::IDENTITY,
    });
    // Keep the animation identical when comparing angles and transparency modes.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));

    for (name, position, oit) in [
        ("above", Vec3::new(0.0, 7.0, 10.0), true),
        ("near-edge", Vec3::new(0.0, 1.7, 12.0), true),
        ("near-edge-below", Vec3::new(0.0, -1.7, 12.0), true),
        ("edge-on", Vec3::new(0.0, 0.0, 12.0), true),
        ("below", Vec3::new(0.0, -7.0, 10.0), true),
        ("face-on", Vec3::new(0.0, 12.0, 0.01), true),
        ("detail", Vec3::new(0.0, 5.0, 7.0), true),
        ("cutoff", Vec3::new(0.0, 2.5, 3.5), true),
        ("cutoff-regular", Vec3::new(0.0, 2.5, 3.5), false),
        ("above-regular", Vec3::new(0.0, 7.0, 10.0), false),
    ] {
        let mut camera = app.world_mut().entity_mut(camera);
        camera.insert(Transform::from_translation(position * radius).looking_at(Vec3::ZERO, Vec3::Y));
        if oit {
            camera.insert(OrderIndependentTransparencySettings::default());
        } else {
            camera.remove::<OrderIndependentTransparencySettings>();
        }
        // Allow assets, render extraction, and newly selected pipelines to settle.
        for _ in 0..40 {
            update(&mut app);
        }
        capture(&mut app, &target, output.join(format!("{name}.png")));
    }
    compare_transparency_modes(&output, "above");
    compare_transparency_modes(&output, "cutoff");
    verify_lensing(&mut app, camera, &target, &output, radius);
    if animate {
        let frames = output.join("frames");
        std::fs::create_dir_all(&frames).expect("Could not create animation directory");
        app.world_mut().entity_mut(camera).insert((
            Transform::from_translation(Vec3::new(0.0, 1.0, 6.0) * radius).looking_at(Vec3::ZERO, Vec3::Y),
            OrderIndependentTransparencySettings::default(),
        ));
        for _ in 0..40 {
            update(&mut app);
        }
        let start_time = app.world().resource::<Time>().elapsed_secs_f64();
        for frame in 0..96 {
            if frame > 0 {
                app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 12.0)));
                update(&mut app);
                app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
            }
            capture(&mut app, &target, frames.join(format!("{frame:04}.png")));
        }
        let elapsed = app.world().resource::<Time>().elapsed_secs_f64() - start_time;
        assert!(
            (elapsed - 95.0 / 12.0).abs() < 0.0001,
            "Animation clock advanced by {elapsed} seconds"
        );
        let first = image::open(frames.join("0000.png")).unwrap().into_rgb8();
        let last = image::open(frames.join("0095.png")).unwrap().into_rgb8();
        assert!(first.as_raw() != last.as_raw(), "Animation did not advance");
        println!("PASS animation: captured 96 changing frames at 12 fps");
    }
    println!("Black hole screenshots saved to {} (radius: {radius})", output.display());
}
