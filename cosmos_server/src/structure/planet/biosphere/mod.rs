//! Represents how a planet will be generated

use bevy::prelude::{App, Commands, Component, Entity, IntoSystemConfig, OnUpdate, Query, With};

use crate::{
    persistence::{
        loading::{begin_loading, done_loading, NeedsLoaded},
        saving::{begin_saving, done_saving, NeedsSaved},
        SerializedData,
    },
    state::GameState,
};

use super::generation::planet_generator::check_needs_generated_system;

pub mod grass_biosphere;
pub mod test_all_stone_biosphere;

/// This has to be redone.
pub trait TGenerateChunkEvent {
    /// Creates the generate chunk event
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self;
}

/// This has to be redone.
pub trait TBiosphere<T: Component, E: TGenerateChunkEvent> {
    /// Gets the marker component used to flag this planet's type
    fn get_marker_component(&self) -> T;
    /// Gets a component for this specific generate chunk event
    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity)
        -> E;
}

/// Use this to register a biosphere
///
/// T: The biosphere's marker component type
/// E: The biosphere's generate chunk event type
pub fn register_biosphere<
    T: Component + Default,
    E: Send + Sync + 'static + TGenerateChunkEvent,
>(
    app: &mut App,
    biosphere_id: &'static str,
) {
    app.add_event::<E>().add_systems((
        (|mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<T>)>| {
            for mut sd in query.iter_mut() {
                sd.serialize_data(biosphere_id.to_string(), &true);
            }
        })
        .after(begin_saving)
        .before(done_saving),
        (|query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands| {
            for (entity, sd) in query.iter() {
                if sd.deserialize_data::<bool>(biosphere_id).unwrap_or(false) {
                    commands.entity(entity).insert(T::default());
                }
            }
        })
        .after(begin_loading)
        .before(done_loading),
        check_needs_generated_system::<E, T>.in_set(OnUpdate(GameState::Playing)),
    ));
}

pub(super) fn register(app: &mut App) {
    grass_biosphere::register(app);
    test_all_stone_biosphere::register(app);
}
