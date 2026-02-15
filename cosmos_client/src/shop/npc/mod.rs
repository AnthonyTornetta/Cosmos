use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{netty::client::LocalPlayer, npc::shop::ShopNpc, state::in_gameplay_state};

use crate::{
    entities::player::PersonMesh,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::LookingAt,
};

fn on_add_npc(
    mut commands: Commands,
    q_shop_npc: Query<Entity, Added<ShopNpc>>,
    person_mesh: Res<PersonMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for ent in q_shop_npc.iter() {
        commands.entity(ent).insert((
            MeshMaterial3d(materials.add(StandardMaterial {
                // Makes the local player's body effectively invisible without disabling their shadow (this is stupid)
                base_color: css::PURPLE.into(),
                ..Default::default()
            })),
            Mesh3d(person_mesh.get()),
        ));
    }
}

fn on_interact_with_npc(
    q_looking_at: Query<&LookingAt, With<LocalPlayer>>,
    inputs: InputChecker,
    q_shop_npc: Query<Entity, With<ShopNpc>>,
) {
    if !inputs.check_just_pressed(CosmosInputs::Interact) {
        return;
    }

    let Some(looking_at) = q_looking_at.single().ok().and_then(|x| x.looking_at_entity) else {
        return;
    };

    if !q_shop_npc.contains(looking_at) {
        return;
    }

    info!("Talked to shop npc!");
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (on_add_npc, on_interact_with_npc).run_if(in_gameplay_state).chain());
}
