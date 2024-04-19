use bevy::{
    app::Update,
    asset::{Assets, Handle},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, Changed, With},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::BuildChildren,
    math::{primitives::Sphere, Vec3, Vec4},
    pbr::{AlphaMode, MaterialMeshBundle, PbrBundle, StandardMaterial},
    prelude::App,
    render::{color::Color, mesh::Mesh},
    time::Time,
    transform::components::Transform,
};
use cosmos_core::structure::{shields::Shield, ship::Ship};

use cosmos_core::ecs::NeedsDespawned;

use crate::asset::materials::shield::{ShieldMaterial, ShieldMaterialExtension};

fn on_add_shield(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_changed_shield: Query<(Entity, &mut Shield), Changed<Shield>>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
) {
    for (shield_ent, mut shield) in q_changed_shield.iter_mut() {
        if shield.strength == 0.0 {
            if let Some(emitting_entity) = shield.emitting_entity {
                commands.entity(emitting_entity).insert(NeedsDespawned);
                shield.emitting_entity = None;
            }
        } else {
            if shield.emitting_entity.is_none() {
                let shield_physical = create_shield_entity(shield.radius, &mut commands, &mut meshes, &mut materials);
                shield.emitting_entity = Some(shield_physical);

                commands.entity(shield_physical).set_parent(shield_ent);
            }
        }
    }
}

fn add_shield(mut commands: Commands, q_added_ship: Query<Entity, Added<Ship>>) {
    for ent in q_added_ship.iter() {
        commands.entity(ent).with_children(|p| {
            p.spawn((
                PbrBundle {
                    transform: Transform::from_translation(Vec3::ZERO),
                    ..Default::default()
                },
                Shield {
                    emitting_entity: None,
                    max_strength: 100.0,
                    radius: 20.0,
                    strength: 1.0,
                },
            ));
        });
    }
}

fn update_shield_times(
    time: Res<Time>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    q_shields: Query<&Handle<ShieldMaterial>, With<ShieldRender>>,
) {
    for handle in &q_shields {
        let Some(mat) = materials.get_mut(handle) else {
            continue;
        };

        for ripple in &mut mat.extension.ripples {
            let old = ripple.w;
            ripple.w = (time.elapsed_seconds() * 4.0) % 2.0;
            if old > ripple.w {
                ripple.x = rand::random::<f32>() * 2.0 - 1.0;
                ripple.y = rand::random::<f32>() * 2.0 - 1.0;
                ripple.z = rand::random::<f32>() * 2.0 - 1.0;
            }
        }
    }
}

#[derive(Component)]
struct ShieldRender;

fn create_shield_entity(radius: f32, commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<ShieldMaterial>) -> Entity {
    commands
        .spawn((
            Name::new("Rendered Shield"),
            ShieldRender,
            MaterialMeshBundle {
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                material: materials.add(ShieldMaterial {
                    base: StandardMaterial {
                        // unlit: true,
                        alpha_mode: AlphaMode::Add,
                        base_color: Color::BLUE,
                        ..Default::default()
                    },
                    extension: ShieldMaterialExtension {
                        ripples: [Vec4::new(0.0, 1.0, 0.0, 0.0); 20],
                    },
                }),
                mesh: meshes.add(Sphere::new(radius)),
                ..Default::default()
            },
        ))
        .id()
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (add_shield, on_add_shield, update_shield_times).chain());
}
