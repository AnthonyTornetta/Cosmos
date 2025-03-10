//! Server item logic

use std::time::Duration;

use bevy::{
    app::Update,
    prelude::{App, Commands, Component, Entity, IntoSystemConfigs, Query, Res, With, Without},
    reflect::Reflect,
    time::Time,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::{health::Dead, player::Player},
    inventory::Inventory,
    item::physical_item::PhysicalItem,
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

#[derive(Default, Component, Debug, Reflect, Serialize, Deserialize, Clone, Copy, PartialEq)]
/// The time (in seconds) since this physcal item was created.
struct TimeSinceSpawn(pub f32);

impl IdentifiableComponent for TimeSinceSpawn {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:time_since_spawn"
    }
}

impl DefaultPersistentComponent for TimeSinceSpawn {}

impl DefaultPersistentComponent for PhysicalItem {}

const PHYSICAL_ITEM_LIFETIME: Duration = Duration::from_mins(5);

fn advance_time_since_spawn(
    mut commands: Commands,
    q_needs_time_since_spawn: Query<Entity, (With<PhysicalItem>, Without<TimeSinceSpawn>)>,
    mut q_physical_items: Query<(Entity, &mut TimeSinceSpawn), With<PhysicalItem>>,
    time: Res<Time>,
) {
    for ent in q_needs_time_since_spawn.iter() {
        commands.entity(ent).insert(TimeSinceSpawn::default());
    }

    for (ent, mut time_since_spawn) in q_physical_items.iter_mut() {
        time_since_spawn.0 += time.delta_secs();

        if time_since_spawn.0 > PHYSICAL_ITEM_LIFETIME.as_secs_f32() {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

fn pickup_near_item(
    mut commands: Commands,
    mut q_physical_items: Query<(Entity, &Location, &mut Inventory, &TimeSinceSpawn), With<PhysicalItem>>,
    mut q_players: Query<(&Location, &mut Inventory), (Without<PhysicalItem>, With<Player>, Without<Dead>)>,
) {
    for (item_entity, item_loc, mut item_inv, time_since_spawn) in q_physical_items.iter_mut() {
        if time_since_spawn.0 < 2.0 {
            continue;
        }

        for (player_loc, mut player_inv) in q_players.iter_mut() {
            if !player_loc.is_within_reasonable_range(item_loc) {
                continue;
            }

            if player_loc.distance_sqrd(item_loc).sqrt() > 2.0 {
                continue;
            }

            let Some(is) = item_inv.itemstack_at(0) else {
                continue;
            };

            let (left_over, _) = player_inv.insert_itemstack(is, &mut commands);
            if left_over == 0 {
                commands.entity(item_entity).insert(NeedsDespawned);
                break;
            } else {
                let delta = is.quantity() - left_over;
                item_inv.decrease_quantity_at(0, delta, &mut commands);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<TimeSinceSpawn>(app);
    make_persistent::<PhysicalItem>(app);

    app.add_systems(
        Update,
        (pickup_near_item, advance_time_since_spawn)
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    )
    .register_type::<TimeSinceSpawn>();
}
