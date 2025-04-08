use bevy::prelude::*;

use super::ComsChannel;

fn name_coms_channel(mut commands: Commands, q_added_coms: Query<Entity, (Added<ComsChannel>, Without<Name>)>) {
    for ent in q_added_coms.iter() {
        commands.entity(ent).insert(Name::new("Coms Channel"));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, name_coms_channel);
}
