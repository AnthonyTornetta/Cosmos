use bevy::prelude::*;

use crate::saving::{begin_saving, done_saving, NeedsSaved, SerializedData};

use super::Structure;

fn on_save_structure(mut query: Query<(&mut SerializedData, &Structure), With<NeedsSaved>>) {
    for (mut s_data, structure) in query.iter_mut() {
        println!("Saving structure!");
        s_data.serialize_data("cosmos:structure", structure);
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(on_save_structure.after(begin_saving).before(done_saving));
}
