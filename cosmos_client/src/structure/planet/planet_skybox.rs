use bevy::{
    color::palettes::css,
    light::{NotShadowCaster, NotShadowReceiver},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    transform::TransformSystems,
};
use cosmos_core::{
    netty::client::LocalPlayer,
    physics::location::Location,
    prelude::{Planet, Structure},
    state::GameState,
    structure::planet::{PLANET_SHAPE_POWER, planet_atmosphere::PlanetAtmosphere},
    universe::star::Star,
};

#[derive(Component)]
struct PlanetSkybox;

const SKYSPHERE_RADIUS: f32 = 5_000_000.0;
const SKY_SHADER: &str = "cosmos/shaders/planet_sky.wesl";

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct PlanetSkyboxMaterialExtension {
    #[uniform(100)]
    sky: SkyParameters,
}

/// Keep this layout in sync with `planet_sky_model.wesl`.
#[derive(Clone, Copy, Debug, ShaderType)]
struct SkyParameters {
    atmosphere_color: Vec4,
    up_density: Vec4,
    sun_direction: Vec4,
    sun_color: Vec4,
}

impl Default for SkyParameters {
    fn default() -> Self {
        Self {
            atmosphere_color: Color::from(css::SKY_BLUE).to_linear().to_vec4(),
            up_density: Vec3::Y.extend(0.0),
            sun_direction: Vec3::Y.extend(0.0),
            sun_color: Vec4::ONE,
        }
    }
}

fn sky_parameters(color: Color, local_position: Vec3, radius: f32, rotation: Quat, sun: Option<(Vec3, Color)>) -> SkyParameters {
    // The gradient of the terrain's Lp distance field gives a continuous local up,
    // including on the rounded edges and corners. Normalize before exponentiation
    // to avoid overflow at large world coordinates.
    let max_axis = local_position.abs().max_element();
    let normal = if max_axis > 0.0 {
        let p = local_position / max_axis;
        (p.signum() * p.abs().powf(PLANET_SHAPE_POWER - 1.0)).normalize_or_zero()
    } else {
        Vec3::Y
    };
    let up = rotation * normal;
    let altitude = (Planet::surface_distance(local_position) - radius).max(0.0);
    let scale_height = (radius * 0.08).max(64.0);
    let height = altitude / scale_height;
    // Exponential thinning followed by a smooth cutoff, with no visible altitude seam.
    let fade = ((height - 4.0) / 2.0).clamp(0.0, 1.0);
    let density = (-height).exp() * (1.0 - fade * fade * (3.0 - 2.0 * fade)) * color.alpha().clamp(0.0, 1.0);
    let mut sky = SkyParameters {
        atmosphere_color: color.to_linear().to_vec4(),
        up_density: up.extend(density),
        ..Default::default()
    };
    if let Some((direction, color)) = sun {
        let direction = direction.normalize_or_zero();
        if direction != Vec3::ZERO {
            let light = color.to_linear().to_vec4().truncate();
            // Stars are emissive HDR meshes; only their chromaticity belongs in the sky tint.
            sky.sun_color = (light / light.max_element().max(0.001)).extend(1.0);
            sky.sun_direction = direction.extend(1.0);
        }
    }
    sky
}

impl MaterialExtension for PlanetSkyboxMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        SKY_SHADER.into()
    }

    fn enable_oit() -> bool {
        // OIT packs reverse-Z depth into 24 bits. At skybox distances this rounds to
        // zero, where translucent fragments fail the comparison with the clear depth.
        // Regular alpha blending retains the depth buffer's floating-point precision.
        false
    }
}

type PlanetSkyboxMaterial = ExtendedMaterial<StandardMaterial, PlanetSkyboxMaterialExtension>;

fn spawn_planet_skysphere(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<PlanetSkyboxMaterial>>, mut commands: Commands) {
    commands.spawn((
        PlanetSkybox,
        DespawnOnExit(GameState::Playing),
        Name::new("Planet skybox"),
        NotShadowCaster,
        NotShadowReceiver,
        Mesh3d(meshes.add(Sphere { radius: SKYSPHERE_RADIUS })),
        MeshMaterial3d(materials.add(PlanetSkyboxMaterial {
            base: StandardMaterial {
                unlit: true,
                base_color: css::SKY_BLUE.into(),
                alpha_mode: AlphaMode::Blend,
                fog_enabled: false,
                ..Default::default()
            },
            extension: PlanetSkyboxMaterialExtension {
                sky: SkyParameters::default(),
            },
        })),
        Transform {
            // By setting the scale to -1, the model will be inverted, which is good since we
            // want to see it while being inside of it.
            scale: Vec3::NEG_ONE,
            ..Default::default()
        },
        Visibility::Hidden,
    ));
}

fn color_planet_skybox(
    q_star_loc: Query<(&Location, &Star)>,
    mut q_planet_skybox: Query<(&mut Visibility, &MeshMaterial3d<PlanetSkyboxMaterial>), With<PlanetSkybox>>,
    q_planets: Query<(&Location, &PlanetAtmosphere, &Structure, &GlobalTransform), With<Planet>>,
    q_player: Query<&Location, With<LocalPlayer>>,
    mut materials: ResMut<Assets<PlanetSkyboxMaterial>>,
) {
    let Ok(player_loc) = q_player.single() else {
        return;
    };

    let Ok((mut vis, skybox_material_handle)) = q_planet_skybox.single_mut() else {
        return;
    };

    let Some((closest_planet_loc, atmosphere, structure, planet_g_trans)) = q_planets
        .iter()
        .min_by(|a, b| a.0.distance_sqrd(player_loc).total_cmp(&b.0.distance_sqrd(player_loc)))
    else {
        *vis = Visibility::Hidden;
        return;
    };

    let planet_rotation = planet_g_trans.rotation();
    let local_position = planet_rotation.inverse() * closest_planet_loc.relative_coords_to(player_loc);
    let planet_radius = structure.block_dimensions().x as f32 / 2.0;
    let sun = q_star_loc
        .iter()
        .min_by(|(a, _), (b, _)| a.distance_sqrd(player_loc).total_cmp(&b.distance_sqrd(player_loc)))
        .map(|(location, star)| (player_loc.relative_coords_to(location), star.color()));
    let sky = sky_parameters(*atmosphere.color(), local_position, planet_radius, planet_rotation, sun);

    if sky.up_density.w < 0.0001 {
        *vis = Visibility::Hidden;
    } else {
        let Some(mut material) = materials.get_mut(skybox_material_handle) else {
            return;
        };

        material.extension.sky = sky;
        *vis = Visibility::Inherited;
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<PlanetSkyboxMaterial>::default())
        .add_systems(
            PostUpdate,
            color_planet_skybox
                .after(TransformSystems::Propagate)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnEnter(GameState::Playing), spawn_planet_skysphere);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::CameraProjection;

    #[test]
    fn sky_up_is_continuous_across_planet_edges_and_follows_rotation() {
        let parameters = |position, rotation| sky_parameters(Color::WHITE, position, 1000.0, rotation, None);
        let face = parameters(Vec3::Y * 1000.0, Quat::IDENTITY);
        assert_eq!(face.up_density.truncate(), Vec3::Y);
        let edge_a = parameters(Vec3::new(1000.0, 999.99, 0.0), Quat::IDENTITY);
        let edge_b = parameters(Vec3::new(999.99, 1000.0, 0.0), Quat::IDENTITY);
        assert!(edge_a.up_density.truncate().dot(edge_b.up_density.truncate()) > 0.99999);
        let corner = parameters(Vec3::splat(-1000.0), Quat::IDENTITY);
        assert!(corner.up_density.truncate().abs_diff_eq(Vec3::NEG_ONE.normalize(), 0.0001));
        let rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let rotated = parameters(Vec3::Y * 1000.0, rotation);
        assert!(rotated.up_density.truncate().abs_diff_eq(Vec3::NEG_X, 0.0001));
        for position in [Vec3::ZERO, Vec3::splat(1.0e25)] {
            assert!(parameters(position, Quat::IDENTITY).up_density.is_finite());
        }
    }

    #[test]
    fn atmospheric_density_uses_cube_surface_altitude_and_fades_to_space() {
        let radius = 1000.0;
        let density = |position| sky_parameters(Color::WHITE, position, radius, Quat::IDENTITY, None).up_density.w;
        let corner_direction = Vec3::ONE.normalize();
        let corner_surface = corner_direction * (radius / Planet::surface_distance(corner_direction));
        assert!((density(corner_surface) - density(Vec3::Y * radius)).abs() < 0.0001);
        let samples: Vec<_> = [0.0, 40.0, 80.0, 160.0, 320.0, 400.0, 480.0]
            .map(|altitude| density(Vec3::Y * (radius + altitude)))
            .to_vec();
        assert_eq!(samples[0], 1.0);
        assert!(samples.windows(2).all(|pair| pair[0] > pair[1]));
        assert_eq!(*samples.last().unwrap(), 0.0);
        assert_eq!(density(Vec3::Y * 1.0e25), 0.0);
    }

    #[test]
    fn sky_preserves_planet_palette_and_uses_the_stars_direction_and_chromaticity() {
        let color = Color::srgba(0.6, 0.3, 0.9, 0.5);
        let sky = sky_parameters(
            color,
            Vec3::Y * 1000.0,
            1000.0,
            Quat::IDENTITY,
            Some((Vec3::new(3.0, -4.0, 0.0), Color::linear_rgb(20.0, 10.0, 5.0))),
        );
        assert_eq!(sky.atmosphere_color, color.to_linear().to_vec4());
        assert_eq!(sky.up_density.w, 0.5);
        assert!(sky.sun_direction.truncate().abs_diff_eq(Vec3::new(0.6, -0.8, 0.0), 0.0001));
        assert_eq!(sky.sun_direction.w, 1.0);
        assert_eq!(sky.sun_color, Vec4::new(1.0, 0.5, 0.25, 1.0));
        let no_star = sky_parameters(color, Vec3::Y * 1000.0, 1000.0, Quat::IDENTITY, None);
        assert_eq!(no_star.sun_direction.w, 0.0);
        let coincident_star = sky_parameters(color, Vec3::Y * 1000.0, 1000.0, Quat::IDENTITY, Some((Vec3::ZERO, Color::WHITE)));
        assert_eq!(coincident_star.sun_direction.w, 0.0);
    }

    #[test]
    fn distant_atmosphere_uses_alpha_blending_without_oit_depth_quantization() {
        // Reproduce the center-of-screen hole: OIT's packed translucent depth is
        // less than the opaque clear depth, despite the sphere being in front of it.
        let projection = PerspectiveProjection::default();
        let depth = projection
            .get_clip_from_view()
            .project_point3(Vec3::new(0.0, 0.0, -SKYSPHERE_RADIUS))
            .z;
        assert!(depth > 0.0);
        let packed_depth = (depth * 0xFFFFFFu32 as f32 + 0.5) as u32;
        assert_eq!(packed_depth, 0);
        let packed_translucent = (packed_depth << 8) | 128;
        let packed_clear = 255;
        assert!(packed_translucent < packed_clear);

        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<PlanetSkyboxMaterial>>()
            .add_systems(Update, spawn_planet_skysphere);
        app.update();
        let material_handle = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<PlanetSkyboxMaterial>, With<PlanetSkybox>>()
            .single(app.world())
            .unwrap();
        let material = app.world().resource::<Assets<PlanetSkyboxMaterial>>().get(material_handle).unwrap();
        assert_eq!(material.base.alpha_mode, AlphaMode::Blend);
        assert!(material.base.unlit);
        assert!(!<PlanetSkyboxMaterial as Material>::enable_oit());
    }
}
