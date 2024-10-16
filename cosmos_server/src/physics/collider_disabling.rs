use bevy::{
    app::Update,
    prelude::{in_state, App, Commands, Entity, IntoSystemConfigs, Parent, Query, With, Without},
};
use cosmos_core::{
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
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
    mut q_entity: Query<(Entity, &Location, Option<&mut DisableRigidBody>), (Without<Player>, Without<Parent>)>,
    q_players: Query<&Location, With<Player>>,
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
        Update,
        disable_colliders
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between)
            .before(DisableRigidBodySet::DisableRigidBodies),
    );
}
