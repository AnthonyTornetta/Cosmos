//! Contains systems and components for the player

use bevy::{color::palettes::css, prelude::*};
use bevy_rapier3d::prelude::{ActiveEvents, CoefficientCombineRule, Collider, Friction, LockedAxes, ReadMassProperties, RigidBody};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    netty::client::LocalPlayer,
    persistence::LoadingDistance,
    state::{GameState, in_gameplay_state},
};

use crate::asset::asset_loader::load_assets;

pub mod death;
pub mod player_movement;
pub mod render_distance;
mod teleport;

fn on_add_player(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // mut meshes: ResMut<Assets<Mesh>>,
    q_player: Query<(Entity, &Player, Has<LocalPlayer>), Added<Player>>,
    person_mesh: Res<PersonMesh>,
) {
    for (ent, player, local) in q_player.iter() {
        commands.entity(ent).insert((
            Mesh3d(person_mesh.get()),
            MeshMaterial3d(materials.add(StandardMaterial {
                // Makes the local player's body effectively invisible without disabling their shadow (this is stupid)
                base_color: if local {
                    Srgba {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.1,
                    }
                    .into()
                } else {
                    css::GREEN.into()
                },
                alpha_mode: if local { AlphaMode::Multiply } else { Default::default() },
                unlit: !local,
                ..Default::default()
            })),
            Collider::capsule_y(0.65, 0.25),
            LockedAxes::ROTATION_LOCKED,
            Name::new(format!("Player ({})", player.name())),
            RigidBody::Dynamic,
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            LoadingDistance::new(1, 2),
            ReadMassProperties::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

/// The mesh for a person
#[derive(Resource)]
pub struct PersonMesh(Handle<Mesh>);

impl PersonMesh {
    /// Gets the mesh handle
    pub fn get(&self) -> Handle<Mesh> {
        self.0.clone()
    }
}

pub(super) fn register(app: &mut App) {
    load_assets::<Mesh, PersonMesh, 1>(app, GameState::Loading, ["cosmos/models/misc/person.obj"], |mut cmds, [mesh]| {
        cmds.insert_resource(PersonMesh(mesh.0));
    });

    app.add_systems(FixedUpdate, on_add_player.in_set(FixedUpdateSet::Main).run_if(in_gameplay_state));

    render_distance::register(app);
    player_movement::register(app);
    death::register(app);
    teleport::register(app);
}
