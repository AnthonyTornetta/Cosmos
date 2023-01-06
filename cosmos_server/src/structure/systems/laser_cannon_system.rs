use bevy::{prelude::*, time::Time};
use cosmos_core::structure::systems::{
    energy_storage_system::EnergyStorageSystem, laser_cannon_system::LaserCannonSystem,
};

use crate::state::GameState;

fn update_system(
    mut query: Query<(&mut LaserCannonSystem, &mut EnergyStorageSystem)>,
    time: Res<Time>,
) {
    for (mut lc, mut es) in query.iter_mut() {
        let sec = time.elapsed_seconds();

        if sec - lc.last_shot_time > 0.1 {
            lc.last_shot_time = sec;

            for line in lc.lines.iter() {
                if es.get_capacity() >= line.property.energy_per_shot {
                    es.decrease_energy(line.property.energy_per_shot);

                    println!("PEW!");
                } else {
                    break;
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(update_system));
}
