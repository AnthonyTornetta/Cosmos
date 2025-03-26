use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::{error, warn},
    prelude::{in_state, Has},
};
use cosmos_core::{
    ecs::NeedsDespawned,
    events::structure::StructureEventListenerSet,
    faction::{FactionId, FactionRelation, Factions},
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    projectiles::missile::Missile,
    state::GameState,
    structure::{
        shared::{DespawnWithStructure, MeltingDown},
        ship::{pilot::Pilot, ship_movement::ShipMovementSet},
        StructureTypeSet,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::{
        loading::LoadingSystemSet,
        make_persistent::{make_persistent, DefaultPersistentComponent},
    },
    structure::systems::thruster_system::MaxShipSpeedModifier,
};

use super::{
    combat::{CombatAi, CombatAiSystemSet, Targetting},
    hit_tracking::DifficultyIncreaseOnKill,
    AiControlled,
};

#[derive(Component, Serialize, Deserialize, Clone, Default, Debug, Copy)]
/// A merchant federation controlled ship
pub struct MerchantFederation;

/// TODO: Load this from config
///
/// How much killing a merchant will increase the difficulty.
/// Aka, if you do 100% of the damage, your strength percentage will increase by this percent.
const MERCHANT_DIFFICULTY_INCREASE: f32 = 5.0;

impl IdentifiableComponent for MerchantFederation {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:merchant_federation"
    }
}

impl DefaultPersistentComponent for MerchantFederation {}

const QUEST_NPC_MAX_CHASE_DISTANCE: f32 = 20_000.0;

/// Attempt to maintain a distance of ~500 blocks from closest target
fn handle_quest_npc_targetting(
    mut commands: Commands,
    mut q_merchants: Query<
        (Entity, &Location, &FactionId),
        (With<MerchantFederation>, Without<Missile>, With<AiControlled>), // Without<Missile> fixes ambiguity issues
    >,
    q_targets: Query<(Entity, &Location, Has<MeltingDown>, Option<&FactionId>)>,
    factions: Res<Factions>,
) {
    for (merchant_ent, merchant_loc, my_faction_id) in q_merchants.iter_mut() {
        let Some(quest_npc_faction) = factions.from_id(my_faction_id) else {
            warn!("Quest NPC faction not found!");
            continue;
        };

        let Some((target_ent, _, _, _)) = q_targets
            .iter()
            .filter(|x| x.1.is_within_reasonable_range(merchant_loc))
            .filter(|(_, _, _, faction_id)| {
                quest_npc_faction.relation_with(faction_id.map(|id| factions.from_id(id)).flatten()) == FactionRelation::Enemy
            })
            // add a large penalty for something that's melting down so they prioritize non-melting down things
            .min_by_key(|(_, this_loc, melting_down, _)| {
                // Makes it only target melting down targets if they're the only one nearby
                let melting_down_punishment = if *melting_down { 100_000_000_000_000 } else { 0 };

                this_loc.distance_sqrd(merchant_loc).floor() as u64 + melting_down_punishment
            })
        else {
            continue;
        };

        commands.entity(merchant_ent).insert(Targetting(target_ent));
    }
}

fn add_merchant_ai(mut commands: Commands, q_needs_ai: Query<Entity, (With<MerchantFederation>, Without<CombatAi>)>) {
    for ent in &q_needs_ai {
        let pilot_ent = commands
            .spawn((
                Name::new("Fake merchant pilot"),
                MerchantPilot,
                DespawnWithStructure,
                Pilot { entity: ent },
            ))
            .id();

        let mut ai = CombatAi {
            max_chase_distance: QUEST_NPC_MAX_CHASE_DISTANCE,
            ..Default::default()
        };
        ai.randomize_inaccuracy();

        commands
            .entity(ent)
            .insert((AiControlled, ai, MaxShipSpeedModifier(0.8), Pilot { entity: pilot_ent }))
            .add_child(pilot_ent);
    }
}

fn add_difficuly_increase(
    mut commands: Commands,
    q_merchant: Query<Entity, (With<MerchantFederation>, Without<DifficultyIncreaseOnKill>)>,
) {
    for ent in &q_merchant {
        commands.entity(ent).insert(DifficultyIncreaseOnKill(MERCHANT_DIFFICULTY_INCREASE));
    }
}

fn on_melt_down(
    q_is_merchant: Query<(), With<MerchantPilot>>,
    q_melting_down: Query<(Entity, Option<&Pilot>), (With<MeltingDown>, With<CombatAi>, With<AiControlled>)>,
    mut commands: Commands,
) {
    for (ent, pilot) in &q_melting_down {
        commands.entity(ent).remove::<(CombatAi, AiControlled, MerchantFederation, Pilot)>();

        if let Some(pilot) = pilot {
            if q_is_merchant.contains(pilot.entity) {
                commands.entity(pilot.entity).insert(NeedsDespawned);
            }
        }
    }
}

#[derive(Component)]
struct MerchantPilot;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum MerchantSystemSet {
    MerchantAiLogic,
}

fn apply_quest_npc_faction(
    factions: Res<Factions>,
    mut commands: Commands,
    q_merchant: Query<Entity, (Without<FactionId>, With<MerchantFederation>)>,
) {
    for ent in q_merchant.iter() {
        let Some(quest_npc_faction) = factions.from_name("Merchant Federation") else {
            error!("No merchant federation faction found! Cannot assign quest npc to faction.");
            return;
        };

        commands.entity(ent).insert(*quest_npc_faction.id());
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<MerchantFederation>(app);

    app.configure_sets(
        Update,
        MerchantSystemSet::MerchantAiLogic
            .before(CombatAiSystemSet::CombatAiLogic)
            .in_set(StructureTypeSet::Ship)
            .after(LoadingSystemSet::DoneLoading)
            .after(StructureEventListenerSet::ChangePilotListener),
    )
    .add_systems(
        Update,
        (
            on_melt_down,
            add_merchant_ai,
            add_difficuly_increase,
            apply_quest_npc_faction,
            handle_quest_npc_targetting.before(ShipMovementSet::RemoveShipMovement),
        )
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between)
            .in_set(MerchantSystemSet::MerchantAiLogic)
            .chain(),
    );
}
