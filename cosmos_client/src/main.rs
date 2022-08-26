mod rendering;

use std::cell::RefCell;
use std::rc::Rc;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONS;

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use cosmos_core::block::blocks::{GRASS, STONE};
use cosmos_core::structure::structure::Structure;
use rapier3d::prelude::{RigidBodyBuilder, RigidBodyType};
use rend3::types::{glam, MipmapCount, MipmapSource, Texture};
use rend3::types::glam::Mat4;
use rend3_routine::pbr::{SampleType, Transparency};
use winit::event::VirtualKeyCode;
use crate::glam::Vec3;
use crate::rendering::structure_renderer::{StructureRenderer};


fn create_meshes() -> Vec<rend3::types::Mesh> {
    let rb_builder = RigidBodyBuilder::new(RigidBodyType::Fixed);

    let mut structure = Structure::new(rb_builder.build(), 1, 1, 1);

    let renderer = Rc::new(RefCell::new(StructureRenderer::new(1, 1, 1)));

    structure.add_structure_listener(renderer.clone());

    for z in 0..CHUNK_DIMENSIONS {
        for x in 0..CHUNK_DIMENSIONS {
            let y: f32 = CHUNK_DIMENSIONS as f32 - ((x + z) as f32 / 12.0).sin().abs() * 4.0;

            for yy in 0..y.ceil() as usize {
                if yy == y.ceil() as usize - 1 {
                    structure.set_block_at(x, yy, z, &GRASS);
                }
                else {
                    structure.set_block_at(x, yy, z, &STONE);
                }
            }
        }
    }

    renderer.borrow_mut().render(&structure);

    // rust needs this for some reason?
    let res = renderer.borrow_mut().create_meshes();
    res
}

const SAMPLE_COUNT: rend3::types::SampleCount = rend3::types::SampleCount::One;

struct CubeExample {
    object_handle: Vec<rend3::types::ObjectHandle>,
    directional_light_handle: Option<rend3::types::DirectionalLightHandle>,

    last_time: SystemTime,
    x: u64,
    y_rot: f32,

    camera_position: Vec3,
    camera_rotation: Vec3,
}

impl Default for CubeExample {
    fn default() -> Self {
        Self {
            last_time: SystemTime::now(),
            object_handle: Vec::new(),
            directional_light_handle: None,
            x: 0,
            y_rot: 0.0,
            camera_position: Vec3::new(0.0, 0.0, 0.0),
            camera_rotation: Vec3::new(0.0, 0.0, 0.0)
        }
    }
}

impl rend3_framework::App for CubeExample {
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Left;

    fn sample_count(&self) -> rend3::types::SampleCount {
        SAMPLE_COUNT
    }

    fn setup(
        &mut self,
        _window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        _routines: &Arc<rend3_framework::DefaultRoutines>,
        _surface_format: rend3::types::TextureFormat,
    ) {
        let image_stone = image::load_from_memory(include_bytes!("../assets/images/atlas/main.png")).expect("Failed to load texture!").to_rgba8();

        let texture_handler = renderer.add_texture_2d(Texture {
            label: None,
            data: image_stone.to_vec(),
            format: rend3::types::TextureFormat::Rgba8UnormSrgb,
            size: glam::UVec2::new(image_stone.dimensions().0, image_stone.dimensions().1),
            mip_count: MipmapCount::ONE,
            mip_source: MipmapSource::Uploaded
        });

        // Add mesh to renderer's world.
        //
        // All handles are refcounted, so we only need to hang onto the handle until we
        // make an object.
        for mesh in create_meshes() {
            let mesh_handle = renderer.add_mesh(mesh);

            // Add PBR material with all defaults except a single color.
            let material = rend3_routine::pbr::PbrMaterial {
                // albedo: rend3_routine::pbr::AlbedoComponent::Value(glam::Vec4::new(1.0, 1.0, 1.0, 1.0)),
                albedo: rend3_routine::pbr::AlbedoComponent::Texture(texture_handler.clone()),
                unlit: true,
                sample_type: SampleType::Nearest,
                transparency: Transparency::Opaque,

                ..rend3_routine::pbr::PbrMaterial::default()
            };
            let material_handle = renderer.add_material(material);

            //glam::Mat4::from_cols()
            // Combine the mesh and the material with a location to give an object.
            let object = rend3::types::Object {
                mesh_kind: rend3::types::ObjectMeshKind::Static(mesh_handle.clone()),
                material: material_handle.clone(),
                transform: Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0))
            };
            // Creating an object will hold onto both the mesh and the material
            // even if they are deleted.
            //
            // We need to keep the object handle alive.
            self.object_handle.push(renderer.add_object(object));
        }

        // Create a single directional light
        //
        // We need to keep the directional light handle alive.
        self.directional_light_handle = Some(renderer.add_directional_light(rend3::types::DirectionalLight {
            color: glam::Vec3::ONE,
            intensity: 10.0,
            // Direction will be normalized
            direction: glam::Vec3::new(-1.0, -4.0, 2.0),
            distance: 400.0,
        }));
    }

    fn handle_event(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        base_rendergraph: &rend3_routine::base::BaseRenderGraph,
        surface: Option<&Arc<rend3::types::Surface>>,
        resolution: glam::UVec2,
        event: rend3_framework::Event<'_, ()>,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
    ) {
        let now = SystemTime::now();

        self.x += 1;
        // self.y_rot += 0.001;

        renderer.set_object_transform(&self.object_handle[0], glam::Mat4::from_euler(glam::EulerRot::XYZ, 0.0, self.y_rot, 0.0));

        if now.duration_since(self.last_time).unwrap() > Duration::from_secs(1)
        {
            self.last_time = now;

            println!("UPS: {}", self.x);
            self.x = 0;
        }

        let view_location = glam::Vec3::new(self.camera_position.x, self.camera_position.y, self.camera_position.z);
        let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, self.camera_rotation.x, self.camera_rotation.y, self.camera_rotation.z);
        let view = view * glam::Mat4::from_translation(-view_location);

        // Set camera's location
        renderer.set_camera_data(rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective { vfov: 90.0, near: 0.1 },
            view,
        });

        match event {
            // Close button was clicked, we should close.
            rend3_framework::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                control_flow(winit::event_loop::ControlFlow::Exit);
            }
            rend3_framework::Event::MainEventsCleared => {
                window.request_redraw();
            }
            // Render!
            rend3_framework::Event::RedrawRequested(_) => {
                // Get a frame
                let frame = rend3::util::output::OutputFrame::Surface {
                    surface: Arc::clone(surface.unwrap()),
                };
                // Ready up the renderer
                let (cmd_bufs, ready) = renderer.ready();

                // Lock the routines
                let pbr_routine = rend3_framework::lock(&routines.pbr);
                let tonemapping_routine = rend3_framework::lock(&routines.tonemapping);

                // Build a rendergraph
                let mut graph = rend3::graph::RenderGraph::new();

                // Add the default rendergraph without a skybox
                base_rendergraph.add_to_graph(
                    &mut graph,
                    &ready,
                    &pbr_routine,
                    None,
                    &tonemapping_routine,
                    resolution,
                    SAMPLE_COUNT,
                    glam::Vec4::ZERO,
                    //glam::Vec4::new(0.10, 0.05, 0.10, 1.0), // Nice scene-referred purple
                );

                // Dispatch a render using the built up rendergraph!
                graph.execute(renderer, frame, cmd_bufs, &ready);

                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            rend3_framework::Event::WindowEvent {
                event: winit::event::WindowEvent::KeyboardInput { input, .. } ,
                ..
            } => {
                if input.virtual_keycode.is_some()
                {
                    match input.virtual_keycode.unwrap()
                    {
                        VirtualKeyCode::W => {
                            self.camera_position.z += 0.1;
                        }
                        VirtualKeyCode::S => {
                            self.camera_position.z -= 0.1;
                        }
                        VirtualKeyCode::A => {
                            self.camera_position.x -= 0.1;
                        }
                        VirtualKeyCode::D => {
                            self.camera_position.x += 0.1;
                        }
                        VirtualKeyCode::Space => {
                            self.camera_position.y += 0.1;
                        }
                        VirtualKeyCode::LShift => {
                            self.camera_position.y -= 0.1;
                        }
                        VirtualKeyCode::Left => {
                            self.camera_rotation.y -= 0.01;
                        }
                        VirtualKeyCode::Right => {
                            self.camera_rotation.y += 0.01;
                        }
                        VirtualKeyCode::Up => {
                            self.camera_rotation.x += 0.01;
                        }
                        VirtualKeyCode::Down => {
                            self.camera_rotation.x -= 0.01;
                        }
                        _ => {

                        }
                    }
                }

            }
            // Other events we don't care about
            _ => {}
        }
    }
}

fn main() {
    let app = CubeExample::default();
    rend3_framework::start(
        app,
        winit::window::WindowBuilder::new()
            .with_title("cube-example")
            .with_maximized(true),
    );
}
