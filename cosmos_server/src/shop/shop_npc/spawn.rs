use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    netty::sync::IdentifiableComponent,
    npc::shop::ShopNpc,
    persistence::LoadingDistance,
    physics::location::{Location, SetPosition},
};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::persistence::{
    loading::LoadingBlueprintSystemSet,
    make_blueprintable::make_blueprintable,
    make_persistent::{DefaultPersistentComponent, make_persistent},
};

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct NeedsShopNpcSpawned;

impl IdentifiableComponent for NeedsShopNpcSpawned {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:needs_shop_npc_spawned"
    }
}

impl DefaultPersistentComponent for NeedsShopNpcSpawned {}

#[derive(Serialize, Deserialize, Reflect, Default, Clone, Copy)]
pub struct ShopNpcSpawnPoint {
    pub relative_position: Vec3,
    pub rotation: Quat,
}

#[derive(Component, Serialize, Deserialize, Reflect, Default, Clone)]
pub struct ShopNpcSpawnPoints(Vec<ShopNpcSpawnPoint>);

impl ShopNpcSpawnPoints {
    pub fn new(pt: ShopNpcSpawnPoint) -> Self {
        Self(vec![pt])
    }

    pub fn add(&mut self, pt: ShopNpcSpawnPoint) {
        self.0.push(pt);
    }
}

impl IdentifiableComponent for ShopNpcSpawnPoints {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shop_npc_spawn_points"
    }
}

impl DefaultPersistentComponent for ShopNpcSpawnPoints {}

const SPAWN_RANGE: f32 = 5000.0;

fn spawn_shop_npc(
    mut commands: Commands,
    q_players: Query<&Location, With<Player>>,
    q_needs_npc_spawned: Query<(Entity, &Location, Option<&ShopNpcSpawnPoints>), With<NeedsShopNpcSpawned>>,
) {
    for (e, loc, spawn_points) in q_needs_npc_spawned.iter() {
        if !q_players.iter().any(|x| x.is_within(SPAWN_RANGE, loc)) {
            info!("Noone close enohught");
            return;
        }

        info!("Spawning shop npc!");

        let point = spawn_points
            .and_then(|spawn_points| spawn_points.0.choose(&mut rand::rng()).copied())
            .unwrap_or_else(|| {
                error!("Missing Shop NPC spawn points on structure! Defaulting to 0,0,0");
                Default::default()
            });

        commands.entity(e).remove::<NeedsShopNpcSpawned>().with_children(|p| {
            p.spawn((
                SetPosition::RelativeTo {
                    entity: e,
                    offset: point.relative_position,
                },
                Transform::from_rotation(point.rotation),
                ShopNpc,
                LoadingDistance::new(1, 1),
            ));
        });
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<NeedsShopNpcSpawned>(app);
    make_blueprintable::<ShopNpcSpawnPoints>(app);
    make_persistent::<ShopNpcSpawnPoints>(app);

    app.add_systems(FixedUpdate, spawn_shop_npc.after(LoadingBlueprintSystemSet::DoneLoadingBlueprints));
}
