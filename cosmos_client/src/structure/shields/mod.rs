use std::time::Duration;

use bevy::{
    app::Update,
    asset::{Assets, Handle},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, Changed},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    math::{Vec3, Vec4},
    pbr::{AlphaMode, StandardMaterial},
    prelude::App,
    render::{
        color::Color,
        mesh::{Mesh, SphereKind, SphereMeshBuilder},
        view::{Visibility, VisibilityBundle},
    },
    time::Time,
};
use cosmos_core::structure::shields::Shield;

use crate::asset::materials::shield::{ShieldMaterial, ShieldMaterialExtension};

fn on_change_shield_update_rendering(mut q_changed_shield: Query<(&Shield, &mut Visibility), Changed<Shield>>) {
    for (shield, mut visibility) in q_changed_shield.iter_mut() {
        if shield.strength == 0.0 {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        } else {
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
    }
}

const MAX_ANIMATION_DURATION: Duration = Duration::from_secs(2);
const MAX_ANIMATIONS: usize = 20;

fn update_shield_times(
    time: Res<Time>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    mut q_shields: Query<(&mut ShieldRender, &Handle<ShieldMaterial>)>,
) {
    for (mut shield_render, handle) in &mut q_shields {
        let Some(mat) = materials.get_mut(handle) else {
            continue;
        };

        for (ripple, hit) in mat.extension.ripples.iter_mut().zip(shield_render.hit_locations.iter_mut()) {
            let Some((hit_point, hit_time)) = hit else {
                if ripple.w >= 0.0 {
                    ripple.w = -1.0;
                }
                continue;
            };

            ripple.w = *hit_time;

            ripple.x = hit_point.x;
            ripple.y = hit_point.y;
            ripple.z = hit_point.z;

            *hit_time += time.delta_seconds();

            if *hit_time > MAX_ANIMATION_DURATION.as_secs_f32() {
                *hit = None;
            }
        }
    }
}

#[derive(Component, Default)]
pub struct ShieldRender {
    hit_locations: [Option<(Vec3, f32)>; MAX_ANIMATIONS],
}

impl ShieldRender {
    pub fn add_hit_point(&mut self, point: Vec3) {
        if let Some(entry) = self.hit_locations.iter_mut().find(|x| x.is_none()) {
            *entry = Some((point.normalize_or_zero(), 0.0));
        }
    }
}

fn on_add_shield_create_rendering(
    q_shield_added: Query<(Entity, &Shield), Added<Shield>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    mut commands: Commands,
) {
    for (shield_entity, shield) in q_shield_added.iter() {
        commands.entity(shield_entity).insert((
            Name::new("Shield"),
            ShieldRender::default(),
            VisibilityBundle::default(),
            materials.add(ShieldMaterial {
                base: StandardMaterial {
                    // unlit: true,
                    alpha_mode: AlphaMode::Add,
                    base_color: Color::BLUE,
                    ..Default::default()
                },
                extension: ShieldMaterialExtension {
                    ripples: [Vec4::new(0.0, 0.0, 0.0, -1.0); MAX_ANIMATIONS],
                },
            }),
            meshes.add(SphereMeshBuilder::new(shield.radius, SphereKind::Uv { sectors: 256, stacks: 256 }).build()),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_shield_create_rendering,
            on_change_shield_update_rendering,
            update_shield_times,
        )
            .chain(),
    );
}
