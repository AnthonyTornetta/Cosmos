use std::time::Duration;

use bevy::{asset::LoadState, prelude::*};
use bevy_hanabi::prelude::*;

use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    ecs::NeedsDespawned,
    physics::location::Location,
    projectiles::missile::{Explosion, ExplosionSystemSet},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, AudioEmitterBundle, CosmosAudioEmitter, DespawnOnNoEmissions},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

#[derive(Component)]
struct TimeAlive(f32);

#[derive(Component)]
struct MaxTimeAlive(Duration);

fn track_time_alive(mut commands: Commands, time: Res<Time>, mut q_time_alive: Query<(Entity, &mut TimeAlive, Option<&MaxTimeAlive>)>) {
    for (ent, mut time_alive, max_time) in &mut q_time_alive {
        time_alive.0 += time.delta_seconds();

        if let Some(max_time) = max_time {
            if time_alive.0 >= max_time.0.as_secs_f32() {
                commands.entity(ent).insert(NeedsDespawned);
            }
        }
    }
}

#[derive(Component)]
struct ExplosionParticle;

fn respond_to_explosion(
    mut commands: Commands,
    q_local_player: Query<&GlobalTransform, With<LocalPlayer>>,
    q_explosions: Query<(Entity, &Location, &GlobalTransform), Added<Explosion>>,
    audio: Res<Audio>,
    audio_sources: Res<ExplosionAudio>,
    particle_effect: Res<ExplosionParticleEffect>,
) {
    let Ok(local_g_trans) = q_local_player.get_single() else {
        return;
    };

    for (ent, explosion_loc, g_trans) in q_explosions.iter() {
        commands.entity(ent).insert(NeedsDespawned);

        let handle = audio_sources.0[rand::random::<usize>() % audio_sources.0.len()].clone_weak();

        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone()).with_volume(0.0).handle();

        commands.spawn((
            Name::new("Explosion particle"),
            *explosion_loc,
            TimeAlive(0.0),
            MaxTimeAlive(MAX_PARTICLE_LIFETIME),
            ExplosionParticle,
            ParticleEffectBundle {
                effect: ParticleEffect::new(particle_effect.0.clone_weak()),
                transform: Transform::from_translation(g_trans.translation()).looking_at(local_g_trans.translation(), Vec3::Y),
                ..Default::default()
            },
        ));
        commands.spawn((
            Name::new("Explosion sound"),
            DespawnOnNoEmissions,
            *explosion_loc,
            AudioEmitterBundle {
                emitter: CosmosAudioEmitter::with_emissions(vec![AudioEmission {
                    instance: playing_sound,
                    handle,
                    max_distance: 200.0,
                    peak_volume: 1.0,
                    ..Default::default()
                }]),
                transform: TransformBundle::from_transform(Transform::from_translation(g_trans.translation())),
            },
        ));
    }
}

/// For some reason, hanabi's auto start doesnt work on the very first particle created, but every subsiquent one.
/// This fixes that issue.
///
/// Note that this needs to be run *before* the explosion particle creation system, for some reason
fn start_explosion_particle_system(mut q_spawner: Query<&mut EffectSpawner, (Added<ExplosionParticle>, With<ExplosionParticle>)>) {
    for mut effect_spawner in &mut q_spawner {
        effect_spawner.reset();
        effect_spawner.set_active(true);
    }
}

#[derive(Resource)]
struct ExplosionParticleEffect(Handle<EffectAsset>);

const MAX_PARTICLE_LIFETIME: Duration = Duration::from_millis(1200);

fn init_explosion_particle_effect(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    // let gradient = Gradient::default()
    //     .with_key(0.0, Vec4::new(1.0, 0.0, 0.0, 1.0))
    //     .with_key(1.0, Vec4::new(0.0, 0.0, 0.0, 0.0));

    // let mut module = Module::default();

    // let init_pos = SetPositionSphereModifier {
    //     center: module.lit(Vec3::ZERO),
    //     radius: module.lit(1.0),
    //     dimension: ShapeDimension::Surface,
    // };

    // let init_vel = SetVelocitySphereModifier {
    //     center: module.lit(Vec3::ZERO),
    //     speed: module.lit(6.0),
    // };

    // let lifetime = module.lit(10.0);
    // let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // let accel = module.lit(Vec3::new(0.0, -3.0, 0.0));
    // let update_accel = AccelModifier::new(accel);

    // let effect = EffectAsset::new(32768, Spawner::rate(5.0.into()), module)
    //     .with_name("TestEffect")
    //     .init(init_pos)
    //     .init(init_vel)
    //     .init(init_lifetime)
    //     .update(update_accel)
    //     .render(ColorOverLifetimeModifier { gradient });

    // let effect_handle = effects.add(effect);

    // stolen & slightly modified from: https://github.com/djeedai/bevy_hanabi/blob/cf16097a7c034c27f36c34ab339941242deddb1f/examples/firework.rs

    let mut color_gradient1 = Gradient::new();
    color_gradient1.add_key(0.0, Vec4::new(4.0, 4.0, 4.0, 1.0));
    color_gradient1.add_key(0.1, Vec4::new(4.0, 4.0, 0.0, 1.0));
    color_gradient1.add_key(0.9, Vec4::new(4.0, 0.0, 0.0, 1.0));
    color_gradient1.add_key(1.0, Vec4::new(4.0, 0.0, 0.0, 0.0));

    let mut size_gradient1 = Gradient::new();
    size_gradient1.add_key(0.0, Vec2::splat(0.1));
    size_gradient1.add_key(0.3, Vec2::splat(0.1));
    size_gradient1.add_key(1.0, Vec2::splat(0.0));

    let writer = ExprWriter::new();

    // Give a bit of variation by randomizing the age per particle. This will
    // control the starting color and starting size of particles.
    let age = writer.lit(0.).uniform(writer.lit(0.2)).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Give a bit of variation by randomizing the lifetime per particle
    let lifetime = writer.lit(0.8).uniform(writer.lit(MAX_PARTICLE_LIFETIME.as_secs_f32())).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Add constant downward acceleration to simulate gravity
    // let accel = writer.lit(Vec3::Y * -8.).expr();
    // let update_accel = AccelModifier::new(accel);

    // Add drag to make particles slow down a bit after the initial explosion
    let drag = writer.lit(5.).expr();
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

    let effect = EffectAsset::new(32768, Spawner::once(2500.0.into(), true), writer.finish())
        .with_name("explosion")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(update_drag)
        // .update(update_accel)
        .render(ColorOverLifetimeModifier { gradient: color_gradient1 })
        .render(SizeOverLifetimeModifier {
            gradient: size_gradient1,
            screen_space_size: false,
        });

    let effect_handle = effects.add(effect);

    commands.insert_resource(ExplosionParticleEffect(effect_handle));
}

#[derive(Resource)]
struct ExplosionAudio(Vec<Handle<AudioSource>>);

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, ExplosionAudio>(
        app,
        GameState::Loading,
        vec![
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
        Update,
        (start_explosion_particle_system, respond_to_explosion)
            .chain()
            .in_set(ExplosionSystemSet::ProcessExplosions)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, track_time_alive)
    .add_systems(OnEnter(GameState::Loading), init_explosion_particle_effect);
}
