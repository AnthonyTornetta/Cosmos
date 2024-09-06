use bevy::{
    color::palettes::css,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};
use cosmos_core::{
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::{Planet, Structure},
    structure::planet::planet_atmosphere::PlanetAtmosphere,
};

use crate::state::game_state::GameState;

#[derive(Component)]
struct PlanetSkybox;

fn spawn_planet_skysphere(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    commands.spawn((
        PlanetSkybox,
        Name::new("Planet skybox"),
        NotShadowCaster,
        NotShadowReceiver,
        PbrBundle {
            mesh: meshes.add(Sphere {
                radius: 1_000_000.0,
                ..Default::default()
            }),
            material: materials.add(StandardMaterial {
                unlit: true,
                base_color: css::SKY_BLUE.into(),
                alpha_mode: AlphaMode::Blend,
                ..Default::default()
            }),
            transform: Transform {
                // By setting the scale to -1, the model will be inverted, which is good since we
                // want to see it while being inside of it.
                scale: Vec3::NEG_ONE,
                ..Default::default()
            },
            visibility: Visibility::Hidden,
            ..Default::default()
        },
    ));
}

fn color_planet_skybox(
    mut q_planet_skybox: Query<(&mut Visibility, &Handle<StandardMaterial>), With<PlanetSkybox>>,
    q_planets: Query<(&Location, &PlanetAtmosphere, &Structure), With<Planet>>,
    q_player: Query<&Location, With<LocalPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_loc) = q_player.get_single() else {
        return;
    };

    let Ok((mut vis, skybox_material_handle)) = q_planet_skybox.get_single_mut() else {
        return;
    };

    let Some((closest_planet_loc, atmosphere, structure)) = q_planets.iter().min_by_key(|x| x.0.distance_sqrd(player_loc).round() as u64)
    else {
        *vis = Visibility::Hidden;
        return;
    };

    let mut color = *atmosphere.color();

    let dist_to_planet = closest_planet_loc.distance_sqrd(player_loc).sqrt();
    let planet_radius = structure.block_dimensions().x as f32 / 2.0;

    // Fades out the alpha has you get further away from the planet
    //
    // 12800 is a random number I made up, feel free to adjust.
    let new_alpha = 12800.0_f32.powf((planet_radius / dist_to_planet).powf(2.0) - 1.0).min(1.0);
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
