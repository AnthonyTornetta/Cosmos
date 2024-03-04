use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        texture::FallbackImage,
        Render, RenderApp, RenderSet,
    },
};

const SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
struct ComputeValues {
    #[storage_texture(0, image_format = R32Float, access = ReadWrite)]
    image: Handle<Image>,
    #[storage(1, visibility(compute))]
    values: Vec<f32>,
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );

    image.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn((
        Name::new("Image cube"),
        PbrBundle {
            transform: Transform::from_xyz(0.0, 0.0, -5.0),
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(image.clone_weak()),
                ..Default::default()
            }),
            ..Default::default()
        },
    ));

    commands.insert_resource(ComputeValues {
        image,
        values: vec![100.0],
    });
}

#[derive(Resource)]
struct CustomComputePipeline {
    bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for CustomComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = ComputeValues::bind_group_layout(render_device);
        let shader = world.resource::<AssetServer>().load("cosmos/shaders/compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        CustomComputePipeline {
            bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

#[derive(Resource)]
struct GameOfLifeImageBindGroup(PreparedBindGroup<()>);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<CustomComputePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    compute_values: Res<ComputeValues>,
    render_device: Res<RenderDevice>,
    fb_image: Res<FallbackImage>,
) {
    // let view = gpu_images.get(&compute_values.image).expect("Unable to get gpu image");

    // let bind_group = render_device.create_bind_group(
    //     None,
    //     &pipeline.bind_group_layout,
    //     &BindGroupEntries::sequential((&view.texture_view, &compute_values.values)),
    // );

    let bind_group = compute_values
        .as_bind_group(&pipeline.bind_group_layout, &render_device, &gpu_images, &fb_image)
        .expect("Nope");

    // let bind_group_layout = render_device.create_bind_group_layout(
    //     None,
    //     &[
    //         BindGroupLayoutEntry {
    //             binding: 1,
    //             count: None,
    //             visibility: ShaderStages::COMPUTE,
    //             ty: BindingType::Buffer {
    //                 has_dynamic_offset: false,
    //                 min_binding_size: Some(NonZeroU64::new(1).unwrap()),
    //                 ty: BufferBindingType::Storage { read_only: false },
    //             },
    //         },
    //         BindGroupLayoutEntry {
    //             binding: 1,
    //             count: None,
    //             visibility: ShaderStages::COMPUTE,
    //             ty: BindingType::Buffer {
    //                 has_dynamic_offset: false,
    //                 min_binding_size: Some(NonZeroU64::new(1).unwrap()),
    //                 ty: BufferBindingType::Storage { read_only: false },
    //             },
    //         },
    //     ],
    // );

    // let storage_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
    //     label: Some("Collatz Conjecture Input"),
    //     contents: gol_image.values.as_slice().as_bytes(),
    //     usage: BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
    // });

    // let bind_group = render_device.create_bind_group(&wgpu::BindGroupDescriptor {
    //     label: None,
    //     layout: &bind_group_layout,
    //     entries: &[wgpu::BindGroupEntry {
    //         binding: 0,
    //         resource: storage_buffer.as_entire_binding(),
    //     }],
    // });

    commands.insert_resource(GameOfLifeImageBindGroup(bind_group));
}

struct GameOfLifeComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GameOfLifeLabel;

enum GameOfLifeState {
    Loading,
    Init,
    Update,
}

struct GameOfLifeNode {
    state: GameOfLifeState,
}

impl Default for GameOfLifeNode {
    fn default() -> Self {
        Self {
            state: GameOfLifeState::Loading,
        }
    }
}

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the game of life image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins(ExtractResourcePlugin::<ComputeValues>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, prepare_bind_group.in_set(RenderSet::PrepareBindGroups));

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(GameOfLifeLabel, GameOfLifeNode::default());
        render_graph.add_node_edge(GameOfLifeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<CustomComputePipeline>();
    }
}

impl render_graph::Node for GameOfLifeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<CustomComputePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GameOfLifeState::Loading => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    self.state = GameOfLifeState::Init;
                }
            }
            GameOfLifeState::Init => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline) {
                    self.state = GameOfLifeState::Update;
                }
            }
            GameOfLifeState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GameOfLifeImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<CustomComputePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, &texture_bind_group.bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GameOfLifeState::Loading => {}
            GameOfLifeState::Init => {
                let init_pipeline = pipeline_cache.get_compute_pipeline(pipeline.init_pipeline).unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            GameOfLifeState::Update => {
                let update_pipeline = pipeline_cache.get_compute_pipeline(pipeline.update_pipeline).unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}

fn printy(res: Res<ComputeValues>) {
    println!("{:?}", res.values);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_plugins(GameOfLifeComputePlugin)
        .add_systems(Update, printy);
}
