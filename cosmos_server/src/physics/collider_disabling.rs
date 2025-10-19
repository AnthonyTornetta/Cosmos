use bevy::prelude::*;
use bevy_rapier3d::prelude::RigidBody;
use cosmos_core::{
    entities::player::Player,
    physics::{
        disable_rigid_body::DisableRigidBody,
        location::{Location, SECTOR_DIMENSIONS, systems::Anchor},
    },
};

const N_SECTORS: f32 = 2.0;
const REASON: &str = "cosmos:far_away";

fn disable_colliders(
    mut commands: Commands,
    mut q_entity: Query<
        (Entity, &Location, Option<&mut DisableRigidBody>),
        (Without<Player>, Without<Anchor>, Without<ChildOf>, With<RigidBody>),
    >,
    q_players: Query<&Location, Or<(With<Anchor>, With<Player>)>>,
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
                info!("ADDING ({loc:?}): {ent:?}, {min_dist:?}");
                disabled_rb.add_reason(REASON);
            } else {
                let mut disabled_rb = DisableRigidBody::default();
                info!("ADDING: ({loc:?}) {ent:?}");
                disabled_rb.add_reason(REASON);
                commands.entity(ent).insert(disabled_rb);
            }
        } else if let Some(mut disabled_rb) = disabled_rb {
            info!("REMOVING: {ent:?}, {min_dist:?}");
            disabled_rb.remove_reason(REASON);
        }
    }
}

pub(super) fn register(app: &mut App) {
    // app.add_systems(
    //     FixedUpdate,
    //     disable_colliders
    //         .run_if(in_state(GameState::Playing))
    //         .in_set(FixedUpdateSet::PostLocationSyncingPostPhysics)
    //         .before(DisableRigidBodySet::DisableRigidBodies),
    // );
}
