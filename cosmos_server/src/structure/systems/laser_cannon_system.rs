use bevy::{prelude::*, time::Time};
use cosmos_core::structure::systems::{
    energy_storage_system::EnergyStorageSystem, laser_cannon_system::LaserCannonSystem,
    StructureSystem, SystemActive, Systems,
};

use crate::state::GameState;

fn update_system(
    mut query: Query<(&mut LaserCannonSystem, &StructureSystem), With<SystemActive>>,
    mut es_query: Query<&mut EnergyStorageSystem>,
    systems: Query<&Systems>,
    time: Res<Time>,
) {
    for (mut cannon_system, system) in query.iter_mut() {
        if let Ok(systems) = systems.get(system.structure_entity) {
            if let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) {
                let sec = time.elapsed_seconds();

                if sec - cannon_system.last_shot_time > 0.1 {
                    cannon_system.last_shot_time = sec;

                    for line in cannon_system.lines.iter() {
                        if energy_storage_system.get_capacity() >= line.property.energy_per_shot {
                            energy_storage_system.decrease_energy(line.property.energy_per_shot);

                            println!("PEW!");
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(update_system));
}
