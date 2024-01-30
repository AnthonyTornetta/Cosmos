use bevy::{
    app::{App, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
    log::info,
};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn on_interact_with_shop(q_structure: Query<&Structure>, blocks: Res<Registry<Block>>, mut ev_reader: EventReader<BlockInteractEvent>) {
    for ev in ev_reader.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = ev.structure_block.block(structure, &blocks);

        if block.unlocalized_name() == "cosmos:shop" {
            info!("Interacted w/ shop");
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_interact_with_shop.after(NetworkingSystemsSet::FlushReceiveMessages));
}
