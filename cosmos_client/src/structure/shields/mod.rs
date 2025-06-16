//! Client-side logic for the rendering of shields

use std::time::Duration;

use bevy::{
    color::palettes::css,
    pbr::NotShadowCaster,
    prelude::*,
    render::mesh::{SphereKind, SphereMeshBuilder},
};
use cosmos_core::{netty::system_sets::NetworkingSystemsSet, structure::shields::Shield};

use crate::{
    asset::materials::shield::{MAX_SHIELD_HIT_POINTS, ShieldMaterial, ShieldMaterialExtension},
    ui::ship_flight::indicators::WaypointSet,
};

#[derive(Component)]
struct OldRadius(f32);

fn on_change_shield_update_rendering(
    mut q_changed_shield: Query<(&Shield, &mut OldRadius, &mut Visibility, &mut Mesh3d), Changed<Shield>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (shield, mut old_radius, mut visibility, mut mesh_handle) in q_changed_shield.iter_mut() {
        if shield.is_enabled() {
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        } else if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }

        if old_radius.0 != shield.radius {
            *mesh_handle = Mesh3d(meshes.add(SphereMeshBuilder::new(shield.radius, SphereKind::Uv { sectors: 256, stacks: 256 }).build()));
            old_radius.0 = shield.radius;
        }
    }
}

const MAX_ANIMATION_DURATION: Duration = Duration::from_secs(2);

fn update_shield_times(
    time: Res<Time>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    mut q_shields: Query<(&mut ShieldRender, &MeshMaterial3d<ShieldMaterial>)>,
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

            *hit_time += time.delta_secs();

            if *hit_time > MAX_ANIMATION_DURATION.as_secs_f32() {
                *hit = None;
            }
        }
    }
}

#[derive(Component)]
/// Contains the info necessary for rendering hits on shield
pub struct ShieldRender {
    hit_locations: [Option<(Vec3, f32)>; MAX_SHIELD_HIT_POINTS],
}

impl Default for ShieldRender {
    fn default() -> Self {
        Self {
            hit_locations: [None; MAX_SHIELD_HIT_POINTS],
        }
    }
}

impl ShieldRender {
    /// Adds a point that was hit on the shield
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
            Visibility::Hidden,
            NotShadowCaster,
            MeshMaterial3d(materials.add(ShieldMaterial {
                base: StandardMaterial {
                    // unlit: true,
                    alpha_mode: AlphaMode::Add,
                    base_color: css::BLUE.into(),
                    ..Default::default()
                },
                extension: ShieldMaterialExtension {
                    ripples: [Vec4::new(0.0, 0.0, 0.0, -1.0); MAX_SHIELD_HIT_POINTS],
                },
            })),
            OldRadius(shield.radius),
            Mesh3d(meshes.add(SphereMeshBuilder::new(shield.radius, SphereKind::Uv { sectors: 256, stacks: 256 }).build())),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_shield_create_rendering,
            on_change_shield_update_rendering.ambiguous_with(WaypointSet::FocusWaypoints),
            update_shield_times,
        )
            .after(NetworkingSystemsSet::Between)
            .chain(),
    );
}
