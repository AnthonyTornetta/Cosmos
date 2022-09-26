use bevy::prelude::{App, EventReader, EventWriter, Query, Res, With};
use cosmos_core::{
    block::blocks::Blocks,
    events::structure::change_pilot_event::ChangePilotEvent,
    structure::{
        ship::{pilot::Pilot, ship::Ship},
        structure::Structure,
    },
};

use super::block_interact_event::BlockInteractEvent;

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    mut change_pilot_event: EventWriter<ChangePilotEvent>,
    s_query: Query<&Structure, With<Ship>>,
    pilot_query: Query<&Pilot>,
    blocks: Res<Blocks>,
) {
    let block = blocks.block_from_id("cosmos:ship_core");

    for ev in interact_events.iter() {
        if ev.block_id == block.id() {
            let maybe_struct = s_query.get(ev.structure_entity);

            // Only works on ships (maybe replace this with pilotable component instead of only checking ships)
            if maybe_struct.is_ok() {
                // Cannot pilot a ship that already has a pilot
                if pilot_query.get(ev.structure_entity).is_err() {
                    change_pilot_event.send(ChangePilotEvent {
                        structure_entity: ev.structure_entity.clone(),
                        pilot_entity: Some(ev.interactor.clone()),
                    });
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(handle_block_event);
}
