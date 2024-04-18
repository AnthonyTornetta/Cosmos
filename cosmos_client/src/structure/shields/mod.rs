use bevy::{
    app::{Startup, Update},
    asset::{Assets, Handle},
    ecs::{
        entity::Entity,
        query::{Added, Changed},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    math::{primitives::Sphere, Vec3},
    pbr::{AlphaMode, MaterialMeshBundle, PbrBundle, StandardMaterial},
    prelude::App,
    render::{color::Color, mesh::Mesh},
    transform::components::Transform,
};
use cosmos_core::structure::{shields::Shield, ship::Ship};

use cosmos_core::ecs::NeedsDespawned;

use crate::asset::materials::shield::{ShieldMaterial, ShieldMaterialExtension};

fn on_add_shield(
    mut commands: Commands,
    shield_material: Res<ShieldMaterialHandle>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_changed_shield: Query<(Entity, &mut Shield), Changed<Shield>>,
) {
    for (shield_ent, mut shield) in q_changed_shield.iter_mut() {
        if shield.strength == 0.0 {
            if let Some(emitting_entity) = shield.emitting_entity {
                commands.entity(emitting_entity).insert(NeedsDespawned);
                shield.emitting_entity = None;
            }
        } else {
            if shield.emitting_entity.is_none() {
                let shield_physical = create_shield_entity(shield.radius, &mut commands, &mut meshes, &shield_material);
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

#[derive(Resource)]
struct ShieldMaterialHandle(Handle<ShieldMaterial>);

fn create_shield_entity(radius: f32, commands: &mut Commands, meshes: &mut Assets<Mesh>, shield_material: &ShieldMaterialHandle) -> Entity {
    commands
        .spawn((MaterialMeshBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            material: shield_material.0.clone_weak(),
            mesh: meshes.add(Sphere::new(radius)),
            ..Default::default()
        },))
        .id()
}

fn create_shield_material(mut commands: Commands, mut materials: ResMut<Assets<ShieldMaterial>>) {
    commands.insert_resource(ShieldMaterialHandle(materials.add(ShieldMaterial {
        base: StandardMaterial {
            // unlit: true,
            alpha_mode: AlphaMode::Add,
            ..Default::default()
        },
        extension: ShieldMaterialExtension { color: Color::AQUAMARINE },
    })));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, create_shield_material)
        .add_systems(Update, (add_shield, on_add_shield).chain());
}
