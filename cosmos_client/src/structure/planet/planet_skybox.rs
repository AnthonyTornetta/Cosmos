use bevy::{
    color::palettes::css,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::{Planet, Structure},
    state::GameState,
    structure::planet::planet_atmosphere::PlanetAtmosphere,
    universe::star::Star,
};

#[derive(Component)]
struct PlanetSkybox;

fn spawn_planet_skysphere(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    commands.spawn((
        PlanetSkybox,
        Name::new("Planet skybox"),
        NotShadowCaster,
        NotShadowReceiver,
        Mesh3d(meshes.add(Sphere { radius: 5_000_000.0 })),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            base_color: css::SKY_BLUE.into(),
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
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
    q_star_loc: Query<&Location, With<Star>>,
    mut q_planet_skybox: Query<(&mut Visibility, &MeshMaterial3d<StandardMaterial>), With<PlanetSkybox>>,
    q_planets: Query<(&Location, &PlanetAtmosphere, &Structure, &GlobalTransform), With<Planet>>,
    q_player: Query<&Location, With<LocalPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_loc) = q_player.single() else {
        return;
    };

    let Ok((mut vis, skybox_material_handle)) = q_planet_skybox.get_single_mut() else {
        return;
    };

    let Some((closest_planet_loc, atmosphere, structure, planet_g_trans)) =
        q_planets.iter().min_by_key(|x| x.0.distance_sqrd(player_loc).round() as u64)
    else {
        *vis = Visibility::Hidden;
        return;
    };

    let mut color = *atmosphere.color();

    let dist_to_planet = closest_planet_loc.distance_sqrd(player_loc).sqrt();
    let planet_radius = structure.block_dimensions().x as f32 / 2.0;

    let closest_star = q_star_loc.iter().min_by_key(|x| x.distance_sqrd(player_loc) as u64);

    // Fades out the alpha has you get further away from the planet
    //
    // 12800 is a random number I made up, feel free to adjust.
    let mut new_alpha = 12800.0_f32.powf((planet_radius / dist_to_planet).powf(2.0) - 1.0).min(1.0);

    if let Some(closest_star) = closest_star {
        let star_direction = Vec3::from(*closest_star - *player_loc).normalize_or_zero();
        let planet_rot = Quat::from_affine3(&planet_g_trans.affine());
        let planet_face_direction = planet_rot
            * Planet::planet_face_relative(planet_rot.inverse() * Vec3::from(*player_loc - *closest_planet_loc))
                .direction()
                .as_vec3();

        let dot = star_direction.dot(planet_face_direction);
        const BEGIN_FADE: f32 = 0.2;
        if dot < BEGIN_FADE {
            new_alpha += 2.0 * (dot - BEGIN_FADE);
            new_alpha = new_alpha.max(0.0);
        }
    } else {
        new_alpha = 0.0;
    }

    color.set_alpha(new_alpha);

    if color.alpha() < 0.001 {
        *vis = Visibility::Hidden;
    } else {
        let Some(material) = materials.get_mut(skybox_material_handle) else {
            return;
        };

        material.base_color = color;
        *vis = Visibility::Inherited;
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        color_planet_skybox
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between),
    )
    .add_systems(OnEnter(GameState::Playing), spawn_planet_skysphere);
}
