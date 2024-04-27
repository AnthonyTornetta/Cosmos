use bevy::{
    app::{App, Update},
    ecs::{event::EventReader, system::Query},
};
use cosmos_core::{projectiles::laser::LaserCollideEvent, structure::shields::Shield};

fn handle_laser_hits(mut ev_reader: EventReader<LaserCollideEvent>, mut q_shield: Query<&mut Shield>) {
    for ev in ev_reader.read() {
        let Ok(mut shield) = q_shield.get_mut(ev.entity_hit()) else {
            continue;
        };

        shield.take_damage(ev.laser_strength());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, handle_laser_hits);
}
