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
    math::Vec4,
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

fn update_shield_times(
    time: Res<Time>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    q_shields: Query<&Handle<ShieldMaterial>, With<ShieldRender>>,
) {
    for handle in &q_shields {
        let Some(mat) = materials.get_mut(handle) else {
            continue;
        };

        // for ripple in &mut mat.extension.ripples {
        //     let old = ripple.w;
        //     ripple.w = time.elapsed_seconds() % 2.0;
        //     if old > ripple.w {
        //         ripple.x = rand::random::<f32>() * 2.0 - 1.0;
        //         ripple.y = rand::random::<f32>() * 2.0 - 1.0;
        //         ripple.z = rand::random::<f32>() * 2.0 - 1.0;
        //     }
        // }
    }
}

#[derive(Component)]
struct ShieldRender;

fn on_add_shield_create_rendering(
    q_shield_added: Query<(Entity, &Shield), Added<Shield>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ShieldMaterial>>,
    mut commands: Commands,
) {
    for (shield_entity, shield) in q_shield_added.iter() {
        commands.entity(shield_entity).insert((
            Name::new("Shield"),
            ShieldRender,
            VisibilityBundle::default(),
            materials.add(ShieldMaterial {
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
            meshes.add(SphereMeshBuilder::new(shield.radius, SphereKind::Uv { sectors: 256, stacks: 256 }).build()),
        ));

        println!("Added shield: {shield_entity:?}");
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
