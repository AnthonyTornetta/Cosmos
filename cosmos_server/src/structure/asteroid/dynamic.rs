use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, systems::Anchor},
    structure::asteroid::MovingAsteroid,
};

use crate::persistence::saving::NeverSave;

const DONT_SAVE_DIST: f32 = 3000.0;

fn dont_save_far(
    mut commands: Commands,
    q_asteroid: Query<(Entity, &Location), With<MovingAsteroid>>,
    q_players: Query<&Location, With<Anchor>>,
) {
    for (ent, loc) in q_asteroid.iter() {
        let Some(d) = q_players.iter().map(|x| x.distance_sqrd(loc)).min_by_key(|x| *x as i32) else {
            continue;
        };

        if d > DONT_SAVE_DIST * DONT_SAVE_DIST {
            commands.entity(ent).insert(NeverSave);
        } else {
            commands.entity(ent).remove::<NeverSave>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        dont_save_far
            .in_set(NetworkingSystemsSet::Between)
            .run_if(on_timer(Duration::from_secs(5))),
    );
}
