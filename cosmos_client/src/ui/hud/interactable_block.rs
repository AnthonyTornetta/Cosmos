use bevy::prelude::*;
use cosmos_core::{
    block::Block, ecs::sets::FixedUpdateSet, netty::client::LocalPlayer, prelude::Structure, registry::Registry,
    structure::ship::pilot::Pilot,
};

use crate::{interactions::block_interactions::LookingAt, ui::crosshair::CrosshairState};

const ID: &str = "cosmos:indicating";

fn on_look_at_interactable_block(
    q_looking_at: Query<&LookingAt, (With<LocalPlayer>, Without<Pilot>)>,
    mut q_crosshair: Query<&mut CrosshairState>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
) {
    let Ok(mut crosshair) = q_crosshair.single_mut() else {
        return;
    };

    let Ok(looking_at) = q_looking_at.single() else {
        crosshair.remove_indicating(ID);
        return;
    };

    let Some(block) = looking_at.looking_at_block else {
        crosshair.remove_indicating(ID);
        return;
    };

    let Ok(structure) = q_structure.get(block.block.structure()) else {
        crosshair.remove_indicating(ID);
        return;
    };

    let block = structure.block_at(block.block.coords(), &blocks);

    if block.interactable() {
        crosshair.request_indicating(ID);
    } else {
        crosshair.remove_indicating(ID);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_look_at_interactable_block.in_set(FixedUpdateSet::PostPhysics));
}
