//! Handles the saving/unloading of reactors

use bevy::prelude::{App, BuildChildren, Commands, Entity, First, IntoSystemConfigs, Query, Update, With};
use cosmos_core::block::multiblock::reactor::{Reactor, Reactors};

use crate::persistence::{
    loading::{begin_loading, done_loading, NeedsLoaded},
    saving::{begin_saving, done_saving, NeedsSaved},
    SerializedData,
};

use super::reactor::ReactorBundle;

fn on_save_reactors(reactor_query: Query<&Reactor>, mut reactors_query: Query<(&Reactors, &mut SerializedData), With<NeedsSaved>>) {
    for (reactors, mut serialized_data) in reactors_query.iter_mut() {
        let reactors = reactors
            .iter()
            .map(|&(_, entity)| {
                reactor_query
                    .get(entity)
                    .expect("Missing reactor component on something that should be a reactor")
            })
            .collect::<Vec<&Reactor>>();

        serialized_data.serialize_data("cosmos:reactors", &reactors);
    }
}

fn on_load_reactors(mut commands: Commands, query: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    for (entity, serialized_data) in query.iter() {
        let Some(reactors_vec) = serialized_data.deserialize_data::<Vec<Reactor>>("cosmos:reactors") else {
            continue;
        };

        let mut reactors = Reactors::default();

        commands
            .entity(entity)
            .with_children(|p| {
                for reactor in reactors_vec {
                    let controller_block = reactor.controller_block();
                    let reactor_entity = p.spawn(ReactorBundle::new(reactor)).id();

                    reactors.add_reactor(reactor_entity, controller_block);
                }
            })
            .insert(reactors);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, on_save_reactors.after(begin_saving).before(done_saving))
        .add_systems(Update, on_load_reactors.after(begin_loading).before(done_loading));
}
