//! Shared systems between different structure types

use bevy::{
    prelude::{App, Commands, Entity, EventWriter, IntoSystemConfigs, IntoSystemSetConfigs, Query, Res, SystemSet, Update, in_state},
    time::Time,
};
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet},
    ecs::NeedsDespawned,
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    registry::Registry,
    state::GameState,
    structure::{Structure, loading::StructureLoadingSet, shared::MeltingDown, ship::pilot::Pilot},
};

pub mod build_mode;
pub mod melt_down;

fn on_melting_down(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Structure, &mut MeltingDown)>,
    mut event_writer: EventWriter<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    time: Res<Time>,
    pilot_query: Query<&Pilot>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
) {
    for (entity, mut structure, mut melting_down) in query.iter_mut() {
        if pilot_query.contains(entity) {
            change_pilot_event.write(ChangePilotEvent {
                structure_entity: entity,
                pilot_entity: None,
            });
        }

        if melting_down.0 >= 1.0 {
            melting_down.0 -= 1.0;

            if let Some(coords) = structure.all_blocks_iter(false).next() {
                structure.remove_block_at(coords, &blocks, Some(&mut event_writer));
            } else {
                commands.entity(entity).insert(NeedsDespawned);
            }
        }

        melting_down.0 += time.delta_secs();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Handles the melting down of ships
pub enum MeltingDownSet {
    /// Triggers the actual meltdown
    StartMeltingDown,
    /// Processes the ship's melting down status
    ProcessMeltingDown,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            MeltingDownSet::StartMeltingDown.in_set(BlockEventsSet::ProcessEvents),
            MeltingDownSet::ProcessMeltingDown
                .in_set(BlockEventsSet::SendEventsForNextFrame)
                .ambiguous_with(BlockEventsSet::SendEventsForNextFrame),
        )
            .chain()
            .after(StructureLoadingSet::StructureLoaded)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(Update, on_melting_down.in_set(MeltingDownSet::ProcessMeltingDown));

    build_mode::register(app);
    melt_down::register(app);
}
