use std::time::Duration;

use bevy::{asset::LoadState, color::palettes::css, platform::collections::HashMap, prelude::*};
use bevy_hanabi::prelude::*;

use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::client::LocalPlayer,
    physics::location::Location,
    projectiles::missile::{Explosion, ExplosionSystemSet, Missile},
    state::GameState,
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    structure::ship::PlayerChildOfChangingSet,
};

#[derive(Component)]
struct ExplosionTimeAlive(f32);

#[derive(Component)]
struct MaxTimeExplosionAlive(Duration);

fn track_time_alive(
    mut commands: Commands,
    time: Res<Time>,
    mut q_time_alive: Query<(Entity, &mut ExplosionTimeAlive, Option<&MaxTimeExplosionAlive>)>,
) {
    for (ent, mut time_alive, max_time) in &mut q_time_alive {
        time_alive.0 += time.delta_secs();

        if let Some(max_time) = max_time
            && time_alive.0 >= max_time.0.as_secs_f32()
        {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

#[derive(Resource)]
struct MissileRenderingInfo(Handle<Mesh>, Handle<StandardMaterial>);

fn create_missile_mesh(asset_server: Res<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>, mut commands: Commands) {
    commands.insert_resource(MissileRenderingInfo(
        asset_server.load("cosmos/models/misc/missile.obj"),
        materials.add(StandardMaterial {
            base_color: css::DARK_GRAY.into(),
            ..Default::default()
        }),
    ));
}

fn on_add_missile(
    mut commands: Commands,
    missile_rendering_info: Res<MissileRenderingInfo>,
    q_added_missile: Query<Entity, Added<Missile>>,
) {
    for ent in &q_added_missile {
        commands.entity(ent).insert((
            Visibility::default(),
            Mesh3d(missile_rendering_info.0.clone_weak()),
            MeshMaterial3d(missile_rendering_info.1.clone_weak()),
        ));
    }
}

#[derive(Component)]
struct ExplosionParticle;

fn color_hash(color: impl Into<Srgba>) -> u32 {
    let Srgba { red, green, blue, alpha } = color.into();
    let (r, g, b, a) = (
        (red * 255.0) as u8,
        (green * 255.0) as u8,
        (blue * 255.0) as u8,
        (alpha * 255.0) as u8,
    );

    u32::from_be_bytes([r, g, b, a])
}

#[derive(Resource, Default)]
struct ParticleEffectsForColor(HashMap<u32, Handle<EffectAsset>>);

fn respond_to_explosion(
    mut commands: Commands,
    q_local_player: Query<&GlobalTransform, With<LocalPlayer>>,
    // q_explosions needs &Transform not &GlobalTransform since &GlobalTransform won't be setup yet.
    q_explosions: Query<(Entity, &Location, &Transform, &Explosion), Added<Explosion>>,
    audio: Res<Audio>,
    audio_sources: Res<ExplosionAudio>,
    mut particles: ResMut<ParticleEffectsForColor>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let Ok(local_g_trans) = q_local_player.single() else {
        return;
    };

    for (ent, explosion_loc, transform, explosion) in q_explosions.iter() {
        let hash = explosion.color.map(color_hash).unwrap_or(0);

        let particle_handle = particles.0.get(&hash).cloned().unwrap_or_else(|| {
            let fx_handle = create_particle_fx(explosion.color, &mut effects);

            let fx_handle_weak = fx_handle.clone();

            particles.0.insert(hash, fx_handle);

            fx_handle_weak
        });

        commands.entity(ent).insert(NeedsDespawned);

        commands.spawn((
            Name::new("Explosion particle"),
            *explosion_loc,
            ExplosionTimeAlive(0.0),
            MaxTimeExplosionAlive(MAX_PARTICLE_LIFETIME),
            ExplosionParticle,
            ParticleEffect::new(particle_handle),
            // Makes the particles appear 3d by looking them at the player
            Transform::from_translation(transform.translation).looking_at(local_g_trans.translation(), Vec3::Y),
        ));

        let audio_handle = audio_sources.0[rand::random::<u64>() as usize % audio_sources.0.len()].clone_weak();

        let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.clone()).with_volume(0.0).handle();

        commands.spawn((
            Name::new("Explosion sound"),
            DespawnOnNoEmissions,
            *explosion_loc,
            CosmosAudioEmitter::with_emissions(vec![AudioEmission {
                instance: playing_sound,
                handle: audio_handle,
                max_distance: 200.0,
                peak_volume: 1.0,
                ..Default::default()
            }]),
            Transform::from_translation(transform.translation),
        ));
    }
}

// /// For some reason, hanabi's auto start doesnt work on the very first particle created, but every subsiquent one.
// /// This fixes that issue.
// ///
// /// Note that this needs to be run *before* the explosion particle creation system, for some reason
// fn start_explosion_particle_system(mut q_spawner: Query<&mut EffectInitializers, (Added<ExplosionParticle>, With<ExplosionParticle>)>) {
//     for mut effect_spawner in &mut q_spawner {
//         effect_spawner.reset();
//         effect_spawner.set_active(true);
//     }
// }

const MAX_PARTICLE_LIFETIME: Duration = Duration::from_millis(1200);

fn create_particle_fx(color: Option<Color>, effects: &mut Assets<EffectAsset>) -> Handle<EffectAsset> {
    // stolen & slightly modified from: https://github.com/djeedai/bevy_hanabi/blob/cf16097a7c034c27f36c34ab339941242deddb1f/examples/firework.rs

    let mut color_gradient1 = Gradient::new();
    if let Some(color) = color {
        let col_vec = Srgba::from(color).to_vec4();
        color_gradient1.add_key(0.0, col_vec * Vec4::new(4.0, 4.0, 4.0, 1.0));
        color_gradient1.add_key(0.1, col_vec * Vec4::new(3.0, 3.0, 3.0, 1.0));
        color_gradient1.add_key(0.9, col_vec * Vec4::new(2.0, 2.0, 2.0, 1.0));
        color_gradient1.add_key(1.0, col_vec * Vec4::new(2.0, 2.0, 2.0, 0.0));
    } else {
        color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
        color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
        color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
        color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));
    }

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.0, Vec3::splat(0.2));
    size_gradient1.add_key(0.3, Vec3::splat(0.2));
    size_gradient1.add_key(1.0, Vec3::splat(0.0));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.2)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.8).uniform(writer.lit(MAX_PARTICLE_LIFETIME.as_secs_f32())).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(10.).expr();
    let update_drag = LinearDragModifier::new(drag);

    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(2.).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Give a bit of variation by randomizing the initial speed
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: (writer.rand(ScalarType::Float) * writer.lit(20.) + writer.lit(20.)).expr(),
    };

    let effect = EffectAsset::new(32768, SpawnerSettings::once(1250.0.into()), writer.finish())
        .with_name("explosion")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(update_drag)
        .with_simulation_space(SimulationSpace::Local)
        .render(ColorOverLifetimeModifier {
            gradient: color_gradient1,
            ..Default::default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient1,
            screen_space_size: false,
        });

    effects.add(effect)
}

#[derive(Resource)]
struct ExplosionAudio(Vec<Handle<AudioSource>>);

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, ExplosionAudio, 4>(
        app,
        GameState::Loading,
        [
            "cosmos/sounds/sfx/explosion-1.ogg",
            "cosmos/sounds/sfx/explosion-2.ogg",
            "cosmos/sounds/sfx/explosion-3.ogg",
            "cosmos/sounds/sfx/explosion-4.ogg",
        ],
        |mut commands, sounds| {
            let sounds = sounds
                .into_iter()
                .flat_map(|(item, state)| match state {
                    LoadState::Loaded => Some(item),
                    _ => None,
                })
                .collect::<Vec<Handle<AudioSource>>>();

            commands.insert_resource(ExplosionAudio(sounds));
        },
    );

    app.add_systems(
        FixedUpdate,
        // (start_explosion_particle_system, respond_to_explosion)
        respond_to_explosion
            // .chain()
            .in_set(ExplosionSystemSet::ProcessExplosions)
            .ambiguous_with(PlayerChildOfChangingSet::ChangeChildOf)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        on_add_missile.run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    )
    .add_systems(OnEnter(GameState::Loading), create_missile_mesh)
    .add_systems(FixedUpdate, track_time_alive)
    .init_resource::<ParticleEffectsForColor>();
}
