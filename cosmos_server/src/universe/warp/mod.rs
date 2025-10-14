use std::time::Duration;

use bevy::prelude::*;
use cosmos_core::{
    physics::{disable_rigid_body::DisableRigidBody, location::Location},
    universe::warp::{WarpTo, WarpingSet},
};

#[derive(Component, Default)]
struct WarpingTime(f32);

const WARP_DURATION: Duration = Duration::from_secs(5);

const REASON: &str = "cosmos:warping";

fn warp_to(mut q_warp_to: Query<(Entity, &mut Location, Option<&mut DisableRigidBody>, &WarpTo), Added<WarpTo>>, mut commands: Commands) {
    for (ent, mut loc, disable_rb, warp_to) in q_warp_to.iter_mut() {
        let mut ecmds = commands.entity(ent);

        if let Some(mut d_rb) = disable_rb {
            d_rb.add_reason(REASON);
        } else {
            ecmds.insert(DisableRigidBody::new_with_reason(REASON));
        }

        ecmds.insert(WarpingTime(0.0));
        // TODO: Check for a good spot!
        *loc = warp_to.loc;
    }
}

fn finish_warping(mut q_warping: Query<(Entity, &mut DisableRigidBody, &mut WarpingTime)>, time: Res<Time>, mut commands: Commands) {
    for (ent, mut drb, mut warping_time) in q_warping.iter_mut() {
        if warping_time.0 >= WARP_DURATION.as_secs_f32() {
            commands.entity(ent).remove::<WarpingTime>().remove::<WarpTo>();
            drb.remove_reason(REASON);
            continue;
        }
        warping_time.0 += time.delta_secs();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            warp_to.in_set(WarpingSet::StartWarping),
            finish_warping.in_set(WarpingSet::DoneWarping),
        ),
    );
}
