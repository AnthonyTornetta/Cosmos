use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    physics::{
        disable_rigid_body::{DisableRigidBody, DisableRigidBodySet},
        location::{Location, SECTOR_DIMENSIONS},
    },
    state::GameState,
};

const N_SECTORS: f32 = 1.0;

const REASON: &str = "cosmos:far_away";

fn disable_colliders(
    mut commands: Commands,
    mut q_entity: Query<(Entity, &Location, Option<&mut DisableRigidBody>), (Without<LocalPlayer>, Without<ChildOf>)>,
    q_players: Query<&Location, With<LocalPlayer>>,
) {
    for (ent, loc, disabled_rb) in q_entity.iter_mut() {
        let Some(min_dist) = q_players
            .iter()
            .map(|x| x.distance_sqrd(loc))
            .min_by(|a, b| a.partial_cmp(b).expect("Got NaN"))
        else {
            return;
        };

        if min_dist.sqrt() > SECTOR_DIMENSIONS * N_SECTORS {
            if let Some(mut disabled_rb) = disabled_rb {
                disabled_rb.add_reason(REASON);
            } else {
                let mut disabled_rb = DisableRigidBody::default();
                disabled_rb.add_reason(REASON);
                commands.entity(ent).insert(disabled_rb);
            }
        } else if let Some(mut disabled_rb) = disabled_rb {
            disabled_rb.remove_reason(REASON);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        disable_colliders
            .run_if(in_state(GameState::Playing))
            .in_set(FixedUpdateSet::PrePhysics)
            .before(DisableRigidBodySet::DisableRigidBodies),
    );
}
