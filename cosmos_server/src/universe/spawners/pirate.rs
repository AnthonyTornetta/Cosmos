use std::time::Duration;

use bevy::{
    app::{App, Startup, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Commands, Query, Res, Resource},
    },
    math::{Quat, Vec3},
    reflect::Reflect,
    time::{common_conditions::on_timer, Time},
    utils::hashbrown::HashMap,
};
use bevy_rapier3d::{dynamics::PhysicsWorld, plugin::DEFAULT_WORLD_ID};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, Sector, SectorUnit, SECTOR_DIMENSIONS},
};

use crate::{persistence::loading::NeedsBlueprintLoaded, state::GameState};

#[derive(Component)]
pub struct PirateNeedsSpawned(Location);

#[derive(Component)]
pub struct Pirate;

fn on_needs_pirate_spawned(mut commands: Commands, q_needs_pirate_spawned: Query<(Entity, &PirateNeedsSpawned)>) {
    for (ent, pns) in q_needs_pirate_spawned.iter() {
        commands.entity(ent).remove::<PirateNeedsSpawned>().insert((
            Pirate,
            NeedsBlueprintLoaded {
                path: "default_blueprints/pirate/default.bp".into(),
                rotation: Quat::IDENTITY,
                spawn_at: pns.0,
            },
        ));
    }
}

#[derive(Default, Component, Clone, Copy, PartialEq, PartialOrd)]
/// Goes on the player and ensures they don't deal with too many pirates
struct LastPirateSpawn(f64);

fn spawn_pirates(
    mut commands: Commands,
    q_players: Query<(Entity, &Location, Option<&LastPirateSpawn>), With<Player>>,
    time: Res<Time>,
    min_pirate_spawn_time: Res<MinPirateSpawnTime>,
) {
    let mut player_groups: HashMap<Sector, (LastPirateSpawn, Vec<Entity>)> = HashMap::default();

    const MAX_DIST: f32 = SECTOR_DIMENSIONS + 20.0;

    for (player_ent, player_loc, player_last_pirate_spawn) in q_players.iter() {
        let player_last_pirate_spawn = player_last_pirate_spawn.copied().unwrap_or(LastPirateSpawn(-100000.0)); //.unwrap_or_default();

        if let Some(sec) = player_groups
            .keys()
            .find(|&sec| Location::new(Vec3::ZERO, *sec - player_loc.sector).distance_sqrd(&Location::ZERO) <= MAX_DIST * MAX_DIST)
            .copied()
        {
            let (last_pirate_spawn, ents) = player_groups.get_mut(&sec).expect("Confirmed to exist above");

            if player_last_pirate_spawn < *last_pirate_spawn {
                *last_pirate_spawn = player_last_pirate_spawn;
            }

            ents.push(player_ent);
        } else {
            player_groups.insert(player_loc.sector, (player_last_pirate_spawn, vec![player_ent]));
        }
    }

    for (sector, (last_pirate_spawn, player_ents)) in player_groups {
        if !(time.elapsed_seconds_f64() - last_pirate_spawn.0 > min_pirate_spawn_time.0.as_secs_f64()) {
            continue;
        }

        const SPAWN_ODDS: f32 = 0.0; // lower = more likely

        if !(rand::random::<f32>() > SPAWN_ODDS) {
            continue;
        }

        let n_pirates = (rand::random::<f32>() * 3.0) as usize + 1; // 1-3

        let fleet_origin = Location::new(
            Vec3::new(random_coord(), random_coord(), random_coord()),
            sector
                + Sector::new(
                    rand::random::<f32>() as SectorUnit,
                    rand::random::<f32>() as SectorUnit,
                    rand::random::<f32>() as SectorUnit,
                ),
        );

        const SPACING: f32 = 500.0;

        for p_idx in 0..n_pirates {
            let offset = p_idx as f32 * SPACING;

            let loc_here = fleet_origin + Vec3::new(offset, 0.0, 0.0);

            commands.spawn((Name::new("Pirate Ship"), PirateNeedsSpawned(loc_here)));
        }

        for player_ent in player_ents {
            commands.entity(player_ent).insert(LastPirateSpawn(time.elapsed_seconds_f64()));
        }
    }
}

fn random_coord() -> f32 {
    rand::random::<f32>() * SECTOR_DIMENSIONS - SECTOR_DIMENSIONS / 2.0
}

#[derive(Resource, Reflect)]
struct MinPirateSpawnTime(Duration);

fn load_settings(mut commands: Commands) {
    commands.insert_resource(MinPirateSpawnTime(Duration::from_mins(10)));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, print_phys_world)
        .add_systems(Startup, load_settings)
        .add_systems(
            Update,
            (spawn_pirates, on_needs_pirate_spawned)
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(on_timer(Duration::from_secs(10))),
        );
}

fn print_phys_world(mut commands: Commands, q_phsy_world: Query<(Entity, &PhysicsWorld)>) {
    for (ent, p_world) in q_phsy_world.iter() {
        if p_world.world_id == DEFAULT_WORLD_ID {
            println!("!!!!!!!!!!!!!!!!!!");
            commands.entity(ent).log_components();
        }
    }
}
