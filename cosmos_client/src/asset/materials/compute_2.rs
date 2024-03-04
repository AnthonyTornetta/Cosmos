use bevy::{
    app::{App, Update},
    ecs::system::Res,
    reflect::TypePath,
};
use bevy_app_compute::prelude::*;

struct Values {
    vals: Vec<f32>,
}

#[derive(TypePath)]
struct ComputeShaderInstance;

impl ComputeShader for ComputeShaderInstance {
    fn shader() -> ShaderRef {
        "cosmos/shaders/compute.wgsl".into()
    }
}

const SIZE: u32 = 8;
const WORKGROUP_SIZE: u32 = 8;

struct ShaderWorker;

impl ComputeWorker for ShaderWorker {
    fn build(world: &mut bevy::prelude::World) -> AppComputeWorker<Self> {
        const DIMS: usize = (SIZE * SIZE * SIZE) as usize;

        let icrs = vec![1.0; DIMS + 1];
        let vals = vec![0.0; DIMS + 1];

        assert!((SIZE * SIZE * SIZE) % WORKGROUP_SIZE == 0);

        AppComputeWorkerBuilder::new(world)
            .add_storage("icr", &icrs)
            .add_staging("values", &vals)
            .add_pass::<ComputeShaderInstance>(
                [SIZE * SIZE * SIZE / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["icr", "values"],
            )
            .build()
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(AppComputeWorkerPlugin::<ShaderWorker>::default())
        .add_systems(Update, print_value);
}

fn print_value(worker: Res<AppComputeWorker<ShaderWorker>>) {
    if !worker.ready() {
        return;
    }

    let v: Vec<f32> = worker.try_read_vec("values").expect("OH NOEEEE!");

    println!("{v:?}");
    // println!("{} and {}", v[0], v[v.len() - 1]);
}
