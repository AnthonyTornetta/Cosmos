//! Contains client-side logic for black holes.

use bevy::{
    color::palettes::css,
    core_pipeline::{
        FullscreenShader,
        core_3d::graph::{Core3d, Node3d},
    },
    ecs::query::QueryItem,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{AnnulusMeshBuilder, MeshVertexBufferLayoutRef},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d},
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            AsBindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
            ColorWrites, FragmentState, Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
            SpecializedMeshPipelineError, TextureSampleType,
            binding_types::{sampler, texture_2d, texture_depth_2d, uniform_buffer},
        },
        renderer::{RenderContext, RenderDevice},
        view::{ViewDepthTexture, ViewTarget},
    },
    shader::ShaderRef,
    transform::TransformSystems,
};
use cosmos_core::universe::black_hole::BlackHole;

use crate::rendering::MainCamera;

const BLACK_HOLE_SHADER: &str = "cosmos/shaders/black_hole.wgsl";
const BLACK_HOLE_LENS_SHADER: &str = "cosmos/shaders/black_hole_lens.wgsl";
const LENS_RADIUS_MULTIPLIER: f32 = 1.78;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct BlackHoleEffectMaterial {
    /// x selects the effect: 0 = accretion disk, 1 = lensed disk arcs,
    /// 2 = photon ring. y controls intensity.
    #[uniform(0)]
    effect: Vec4,
}

impl Material for BlackHoleEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        BLACK_HOLE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn effect_material(kind: f32, intensity: f32) -> BlackHoleEffectMaterial {
    BlackHoleEffectMaterial {
        effect: Vec4::new(kind, intensity, 0.0, 0.0),
    }
}

fn create_added_black_hole(
    added: Query<(Entity, &BlackHole), Added<BlackHole>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut horizon_materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<BlackHoleEffectMaterial>>,
    mut commands: Commands,
) {
    for (entity, black_hole) in added.iter() {
        let radius = black_hole.radius;
        let disk_mesh = meshes.add(AnnulusMeshBuilder::new(radius * 1.12, radius * 4.2, 256).build());
        let lensed_disk_mesh = meshes.add(Sphere::new(radius * 1.52));
        let photon_ring_mesh = meshes.add(Sphere::new(radius * 1.16));

        commands.entity(entity).insert((
            Name::new("Black Hole Event Horizon"),
            Mesh3d(meshes.add(Sphere { radius })),
            MeshMaterial3d(horizon_materials.add(StandardMaterial {
                base_color: css::BLACK.into(),
                unlit: true,
                double_sided: true,
                ..Default::default()
            })),
            NotShadowCaster,
            NotShadowReceiver,
        ));

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new("Black Hole Accretion Disk"),
                Mesh3d(disk_mesh),
                MeshMaterial3d(effect_materials.add(effect_material(0.0, 1.15))),
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                NotShadowCaster,
                NotShadowReceiver,
            ));
            parent.spawn((
                Name::new("Black Hole Lensed Accretion Disk"),
                Mesh3d(lensed_disk_mesh),
                MeshMaterial3d(effect_materials.add(effect_material(1.0, 1.0))),
                NotShadowCaster,
                NotShadowReceiver,
            ));
            parent.spawn((
                Name::new("Black Hole Photon Ring"),
                Mesh3d(photon_ring_mesh),
                MeshMaterial3d(effect_materials.add(effect_material(2.0, 1.5))),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        });
    }
}

/// Screen-space position of the black hole, extracted with the main camera.
/// The radius is stored as a fraction of viewport height so the shader remains
/// circular at every aspect ratio.
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
struct BlackHoleLensSettings {
    /// xy = viewport-space center, z = radius as a fraction of viewport
    /// height, w = distortion strength.
    screen: Vec4,
    /// x = reverse-Z depth of the near edge of the lens volume.
    depth: Vec4,
}

fn update_black_hole_lens(
    mut commands: Commands,
    black_holes: Query<(&BlackHole, &GlobalTransform)>,
    mut cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&mut BlackHoleLensSettings>), With<MainCamera>>,
) {
    let Ok((camera_entity, camera, camera_transform, current_settings)) = cameras.single_mut() else {
        return;
    };
    let Some((black_hole, black_hole_transform)) = black_holes.iter().next() else {
        commands.entity(camera_entity).remove::<BlackHoleLensSettings>();
        return;
    };
    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };

    let center_world = black_hole_transform.translation();
    let camera_right = camera_transform.rotation() * Vec3::X;
    let lens_radius = black_hole.radius * LENS_RADIUS_MULTIPLIER;
    let edge_world = center_world + camera_right * lens_radius;
    let direction_to_camera = (camera_transform.translation() - center_world).normalize_or_zero();
    let lens_near_world = center_world + direction_to_camera * lens_radius;
    let (Ok(center), Ok(edge)) = (
        camera.world_to_viewport(camera_transform, center_world),
        camera.world_to_viewport(camera_transform, edge_world),
    ) else {
        commands.entity(camera_entity).remove::<BlackHoleLensSettings>();
        return;
    };
    let Some(lens_near_depth) = camera.world_to_ndc(camera_transform, lens_near_world).map(|position| position.z) else {
        commands.entity(camera_entity).remove::<BlackHoleLensSettings>();
        return;
    };

    let settings = BlackHoleLensSettings {
        screen: (center / viewport_size)
            .extend(center.distance(edge) / viewport_size.y)
            .extend(0.18),
        depth: Vec4::new(lens_near_depth, 0.0, 0.0, 0.0),
    };
    if let Some(mut current_settings) = current_settings {
        *current_settings = settings;
    } else {
        commands.entity(camera_entity).insert(settings);
    }
}

struct BlackHoleLensPlugin;

impl Plugin for BlackHoleLensPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<BlackHoleLensSettings>::default(),
            UniformComponentPlugin::<BlackHoleLensSettings>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_black_hole_lens_pipeline)
            .add_render_graph_node::<ViewNodeRunner<BlackHoleLensNode>>(Core3d, BlackHoleLensLabel)
            .add_render_graph_edges(Core3d, (Node3d::Tonemapping, BlackHoleLensLabel, Node3d::EndMainPassPostProcessing));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BlackHoleLensLabel;

#[derive(Default)]
struct BlackHoleLensNode;

impl ViewNode for BlackHoleLensNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static BlackHoleLensSettings,
        &'static DynamicUniformIndex<BlackHoleLensSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, depth_texture, _settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let lens_pipeline = world.resource::<BlackHoleLensPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(lens_pipeline.pipeline_id) else {
            return Ok(());
        };
        let settings_uniforms = world.resource::<ComponentUniforms<BlackHoleLensSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let bind_group = render_context.render_device().create_bind_group(
            "black_hole_lens_bind_group",
            &pipeline_cache.get_bind_group_layout(&lens_pipeline.layout),
            &BindGroupEntries::sequential((
                post_process.source,
                &lens_pipeline.sampler,
                depth_texture.view(),
                settings_binding.clone(),
            )),
        );
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
        });
        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct BlackHoleLensPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn init_black_hole_lens_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "black_hole_lens_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_depth_2d(),
                uniform_buffer::<BlackHoleLensSettings>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor::default());
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("black_hole_lens_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: asset_server.load(BLACK_HOLE_LENS_SHADER),
            targets: vec![Some(ColorTargetState {
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(BlackHoleLensPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

pub(super) fn register(app: &mut App) {
    app.add_plugins((MaterialPlugin::<BlackHoleEffectMaterial>::default(), BlackHoleLensPlugin))
        .add_systems(Update, create_added_black_hole)
        .add_systems(PostUpdate, update_black_hole_lens.after(TransformSystems::Propagate));
}
