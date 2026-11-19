//! Contains client-side logic for black holes.

use bevy::{
    color::palettes::css,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{AnnulusMeshBuilder, MeshVertexBufferLayoutRef},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    shader::ShaderRef,
};
use cosmos_core::universe::black_hole::BlackHole;

const BLACK_HOLE_SHADER: &str = "cosmos/shaders/black_hole.wesl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct AccretionDiskMaterial {
    #[uniform(0)]
    intensity: f32,
}

impl Material for AccretionDiskMaterial {
    fn fragment_shader() -> ShaderRef {
        BLACK_HOLE_SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn enable_oit() -> bool {
        // OIT packs depth into 24 bits, rounding distant disk fragments to the
        // sky's zero depth at the game's scale. Use full-precision depth testing
        // and ordinary premultiplied blending for this material instead.
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

fn create_added_black_hole(
    added: Query<(Entity, &BlackHole), Added<BlackHole>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut horizon_materials: ResMut<Assets<StandardMaterial>>,
    mut disk_materials: ResMut<Assets<AccretionDiskMaterial>>,
    mut commands: Commands,
) {
    for (entity, black_hole) in added.iter() {
        let radius = black_hole.radius;
        let disk_mesh = meshes.add(AnnulusMeshBuilder::new(radius * 1.12, radius * 4.2, 256).build());

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
                MeshMaterial3d(disk_materials.add(AccretionDiskMaterial { intensity: 1.0 })),
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<AccretionDiskMaterial>::default())
        .add_systems(Update, create_added_black_hole);
}
