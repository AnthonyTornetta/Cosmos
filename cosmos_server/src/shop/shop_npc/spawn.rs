use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    netty::sync::IdentifiableComponent,
    npc::shop::ShopNpc,
    persistence::LoadingDistance,
    physics::location::{Location, SetPosition},
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct NeedsShopNpcSpawned;

impl IdentifiableComponent for NeedsShopNpcSpawned {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:needs_shop_npc_spawned"
    }
}

impl DefaultPersistentComponent for NeedsShopNpcSpawned {}

const SPAWN_RANGE: f32 = 5000.0;

fn spawn_shop_npc(
    mut commands: Commands,
    q_players: Query<&Location, With<Player>>,
    q_needs_npc_spawned: Query<(Entity, &Location), With<NeedsShopNpcSpawned>>,
) {
    for (e, loc) in q_needs_npc_spawned.iter() {
        if !q_players.iter().any(|x| x.is_within(SPAWN_RANGE, loc)) {
            return;
        }

        commands.entity(e).remove::<NeedsShopNpcSpawned>().with_children(|p| {
            p.spawn((
                SetPosition::RelativeTo {
                    entity: e,
                    offset: Vec3::ZERO,
                },
                Transform::default(),
                ShopNpc,
                LoadingDistance::new(1, 1),
            ));
        });
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<NeedsShopNpcSpawned>(app);
}
