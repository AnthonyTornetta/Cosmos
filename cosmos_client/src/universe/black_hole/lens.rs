//! A local screen-space approximation of gravitational lensing.

use bevy::{
    core_pipeline::{
        Core3d, Core3dSystems, FullscreenShader,
        core_3d::{main_opaque_pass_3d, main_transparent_pass_3d},
    },
    platform::collections::HashMap,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_resource::{
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            FilterMode, FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat,
            TextureSampleType, TextureUsages,
            binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::{ViewDepthStencilTexture, ViewTarget},
    },
    transform::TransformSystems,
};
use cosmos_core::universe::black_hole::BlackHole;

/// Opt a world camera into black hole lensing. World cameras use `Msaa::Off`.
#[derive(Component)]
pub(crate) struct BlackHoleLensCamera;

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
#[extract_app(RenderApp)]
struct LensSettings {
    /// xy = center in target UVs, z = apparent horizon radius / target height.
    screen: Vec4,
    /// x = near-field reverse-Z depth (f32), w = shared disk animation time.
    depth: Vec4,
    /// UV bounds of the camera viewport within the target.
    viewport: Vec4,
    /// xy = projected disk-up direction, z = far-side angle, w = inclination.
    disk: Vec4,
}

fn update_lens(
    mut commands: Commands,
    holes: Query<(&BlackHole, &GlobalTransform)>,
    time: Res<Time>,
    mut cameras: Query<(
        Entity,
        &Camera,
        &GlobalTransform,
        &Msaa,
        &mut Camera3d,
        Option<&BlackHoleLensCamera>,
        Option<&mut LensSettings>,
    )>,
) {
    for (entity, camera, transform, msaa, mut camera_3d, enabled, current) in &mut cameras {
        let settings = (|| {
            if enabled.is_none() || !camera.is_active || *msaa != Msaa::Off {
                return None;
            }
            let (hole, hole_transform) = holes.iter().next()?;
            let size = camera.logical_target_size()?;
            let viewport = camera.logical_viewport_rect()?;
            let center_world = hole_transform.translation();
            let offset = center_world - transform.translation();
            let distance = offset.length();
            if distance <= hole.radius * 1.05 {
                return None;
            }
            // Project the tangent radius of the sphere, rather than a radius
            // in its center plane. This stays aligned with the shadow up close.
            let tangent_radius = hole.radius * distance / (distance * distance - hole.radius * hole.radius).sqrt();
            let right = transform.rotation() * Vec3::X;
            let center = camera.world_to_viewport(transform, center_world).ok()?;
            let edge = camera.world_to_viewport(transform, center_world + right * tangent_radius).ok()?;
            let near_distance = (distance - hole.radius * 2.8).max(hole.radius * 0.05);
            let near_world = transform.translation() + offset / distance * near_distance;
            let near_depth = camera.world_to_ndc(transform, near_world)?.z;
            let normal = hole_transform.rotation() * Vec3::Y;
            let inclination = normal.dot(-offset / distance);
            let view_normal = transform.rotation().inverse() * normal;
            let mut up = Vec2::new(view_normal.x, -view_normal.y).try_normalize().unwrap_or(Vec2::NEG_Y);
            if inclination < 0.0 {
                up = -up;
            }
            let local_camera = hole_transform.rotation().inverse() * -offset;
            Some(LensSettings {
                screen: (center / size).extend(center.distance(edge) / size.y).extend(0.22),
                depth: Vec4::new(near_depth, 0.0, 0.0, time.elapsed_secs_wrapped()),
                viewport: Vec4::new(
                    viewport.min.x / size.x,
                    viewport.min.y / size.y,
                    viewport.max.x / size.x,
                    viewport.max.y / size.y,
                ),
                disk: up.extend(local_camera.x.atan2(-local_camera.z)).extend(inclination),
            })
        })();
        if let Some(settings) = settings {
            let usage = TextureUsages::from(camera_3d.depth_texture_usages) | TextureUsages::TEXTURE_BINDING;
            if usage != TextureUsages::from(camera_3d.depth_texture_usages) {
                camera_3d.depth_texture_usages = usage.into();
            }
            if let Some(mut current) = current {
                *current = settings;
            } else {
                commands.entity(entity).insert(settings);
            }
        } else if current.is_some() {
            commands.entity(entity).remove::<LensSettings>();
        }
    }
}

#[derive(Resource)]
struct LensPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    descriptor: RenderPipelineDescriptor,
}

fn init_pipeline(mut commands: Commands, device: Res<RenderDevice>, assets: Res<AssetServer>, fullscreen: Res<FullscreenShader>) {
    let layout = BindGroupLayoutDescriptor::new(
        "black_hole_lens_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_depth_2d(),
                uniform_buffer::<LensSettings>(true),
            ),
        ),
    );
    let descriptor = RenderPipelineDescriptor {
        label: Some("black_hole_lens_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: assets.load("cosmos/shaders/black_hole_lens.wesl"),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    };
    commands.insert_resource(LensPipeline {
        layout,
        sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        descriptor,
    });
}

fn lens_pass(
    view: ViewQuery<(&ViewTarget, &ViewDepthStencilTexture, &DynamicUniformIndex<LensSettings>)>,
    lens: Res<LensPipeline>,
    cache: Res<PipelineCache>,
    uniforms: Res<ComponentUniforms<LensSettings>>,
    mut pipelines: Local<HashMap<TextureFormat, CachedRenderPipelineId>>,
    mut context: RenderContext,
) {
    let (target, depth, index) = view.into_inner();
    let pipeline = *pipelines.entry(target.main_texture_format()).or_insert_with(|| {
        let mut descriptor = lens.descriptor.clone();
        descriptor.fragment.as_mut().unwrap().targets[0].as_mut().unwrap().format = target.main_texture_format();
        cache.queue_render_pipeline(descriptor)
    });
    let (Some(pipeline), Some(uniforms)) = (cache.get_render_pipeline(pipeline), uniforms.uniforms().binding()) else {
        return;
    };
    let post_process = target.post_process_write();
    let bind_group = context.render_device().create_bind_group(
        "black_hole_lens_bind_group",
        &cache.get_bind_group_layout(&lens.layout),
        &BindGroupEntries::sequential((
            post_process.source,
            &lens.sampler,
            depth
                .attachment
                .depth_stencil_views()
                .depth_only_view()
                .expect("Camera has a depth view"),
            uniforms,
        )),
    );
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("black_hole_lens_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post_process.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[index.index()]);
    pass.draw(0..3, 0..1);
}

pub(super) fn register(app: &mut App) {
    app.add_plugins((
        ExtractComponentPlugin::<LensSettings>::default(),
        UniformComponentPlugin::<LensSettings>::default(),
    ))
    .add_systems(PostUpdate, update_lens.after(TransformSystems::Propagate));
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(RenderStartup, init_pipeline).add_systems(
            Core3d,
            lens_pass
                .after(main_opaque_pass_3d)
                .before(main_transparent_pass_3d)
                .in_set(Core3dSystems::MainPass),
        );
    }
}
