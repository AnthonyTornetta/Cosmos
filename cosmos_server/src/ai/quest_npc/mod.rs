//! The merchant NPC's AI logic

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
    math::Vec3,
    prelude::{in_state, Changed, EventWriter, Has, Or, Parent, Transform},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    coms::{AiComsType, ComsChannel, RequestedComs},
    ecs::NeedsDespawned,
    entities::EntityId,
    events::structure::StructureEventListenerSet,
    faction::{FactionId, FactionRelation, Factions},
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::Ship,
    projectiles::missile::Missile,
    state::GameState,
    structure::{
        shared::{DespawnWithStructure, MeltingDown},
        ship::{
            pilot::{Pilot, PilotFocused},
            ship_movement::{ShipMovement, ShipMovementSet},
        },
        StructureTypeSet,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    coms::RequestHailFromNpc,
    persistence::{
        loading::LoadingSystemSet,
        make_persistent::{make_persistent, DefaultPersistentComponent},
    },
    structure::systems::thruster_system::MaxShipSpeedModifier,
};

use super::{
    combat::{AiTargetting, CombatAi, CombatAiSystemSet},
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
        (Entity, &Location, &FactionId, &mut MerchantAiState),
        (With<MerchantFederation>, With<CombatAi>, Without<Missile>, With<AiControlled>), // Without<Missile> fixes ambiguity issues
    >,
    q_targets: Query<(Entity, &Location, Has<MeltingDown>, Option<&FactionId>)>,
    factions: Res<Factions>,
) {
    for (merchant_ent, merchant_loc, my_faction_id, mut ai_state) in q_merchants.iter_mut() {
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
            commands.entity(merchant_ent).remove::<AiTargetting>();
            *ai_state = MerchantAiState::default();
            continue;
        };

        commands.entity(merchant_ent).insert(AiTargetting(target_ent));
    }
}

fn add_merchant_ai(
    mut commands: Commands,
    q_needs_ai: Query<Entity, (With<MerchantFederation>, Or<(Without<SaidNoList>, Without<MerchantAiState>)>)>,
) {
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
            .insert((
                AiControlled,
                ai,
                MaxShipSpeedModifier(0.8),
                SaidNoList::default(),
                MerchantAiState::default(),
                Pilot { entity: pilot_ent },
            ))
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

#[derive(Component, Debug, Reflect, Default)]
enum MerchantAiState {
    #[default]
    FindingTarget,
    Fighting,
    Talking,
}

const TALK_DIST: f32 = 1000.0;

fn searching_merchant_ai(
    mut q_merchant: Query<
        (&Location, &mut Transform, &FactionId, &mut MerchantAiState, &mut ShipMovement),
        (With<MerchantFederation>, With<AiControlled>, Without<AiTargetting>),
    >,
    q_targets: Query<(&EntityId, &Location, Option<&FactionId>, &Pilot), (With<Ship>, Without<AiControlled>)>,
    factions: Res<Factions>,
) {
    for (m_loc, mut m_trans, m_fac, mut ai_state, mut ship_movement) in q_merchant.iter_mut() {
        if !matches!(*ai_state, MerchantAiState::FindingTarget) {
            continue;
        }

        let Some(m_fac) = factions.from_id(m_fac) else {
            warn!("Merchant faction not found!");
            continue;
        };

        let target = q_targets
            .iter()
            .filter(|(target_ent, _, target_fac, _)| {
                m_fac.relation_with_entity(target_ent, target_fac.and_then(|x| factions.from_id(x))) != FactionRelation::Enemy
            })
            .min_by_key(|(_, t_loc, _, _)| t_loc.distance_sqrd(m_loc) as i32)
            .map(|(_, t_loc, _, _)| *t_loc);

        let Some(target) = target else {
            continue;
        };

        let dist = target.distance_sqrd(m_loc).sqrt();
        if dist < TALK_DIST {
            *ai_state = MerchantAiState::Talking;
            continue;
        }

        m_trans.look_to((target - *m_loc).absolute_coords_f32(), Vec3::Y);
        ship_movement.movement = Vec3::Z;
    }
}

fn combat_merchant_ai(mut commands: Commands, q_merchant: Query<(Entity, &MerchantAiState), Changed<MerchantAiState>>) {
    for (ent, state) in q_merchant.iter() {
        match state {
            MerchantAiState::Fighting => {
                commands.entity(ent).insert(CombatAi::default());
            }
            _ => {
                commands.entity(ent).remove::<CombatAi>();
            }
        }
    }
}

#[derive(Component, Default)]
struct SaidNoList(Vec<Entity>);

fn talking_merchant_ai(
    mut evw_send_coms: EventWriter<RequestHailFromNpc>,
    mut commands: Commands,
    mut q_merchant: Query<
        (
            Entity,
            &Location,
            &FactionId,
            &mut MerchantAiState,
            &mut ShipMovement,
            &SaidNoList,
            &Velocity,
        ),
        (With<MerchantFederation>, Without<AiTargetting>, With<Pilot>),
    >,
    q_coms: Query<(&ComsChannel, &Parent)>,
    q_targets: Query<
        (
            Entity,
            &EntityId,
            &Location,
            Option<&FactionId>,
            &Pilot,
            &Velocity,
            Has<RequestedComs>,
        ),
        (With<Ship>, Without<AiControlled>),
    >,
    factions: Res<Factions>,
) {
    for (entity, m_loc, m_fac, mut ai_state, mut ship_movement, said_no_list, velocity) in q_merchant.iter_mut() {
        if !matches!(*ai_state, MerchantAiState::Talking) {
            continue;
        }

        let Some(m_fac) = factions.from_id(m_fac) else {
            warn!("Merchant faction not found!");
            continue;
        };

        let coms_with_this = q_coms.iter().filter(|c| c.0.with == entity).collect::<Vec<_>>();

        let (target, needs_coms) = if let Some(target) = coms_with_this.iter().flat_map(|x| q_targets.get(x.1.get())).next() {
            (Some(target), false)
        } else {
            let target = q_targets
                .iter()
                .filter(|x| !said_no_list.0.contains(&x.0))
                // .filter(|x| !coms_with_this.iter().any(|c| c.1.get() == x.0))
                .filter(|(_, target_ent_id, _, target_fac, _, _, _)| {
                    m_fac.relation_with_entity(target_ent_id, target_fac.and_then(|x| factions.from_id(x))) != FactionRelation::Enemy
                })
                .min_by_key(|(_, _, t_loc, _, _, _, _)| t_loc.distance_sqrd(m_loc) as i32);

            (target, true)
        };

        let Some((target_ent, _, target_loc, _, pilot, target_vel, target_requested_coms)) = target else {
            continue;
        };

        let dist = target_loc.distance_sqrd(m_loc).sqrt();
        if dist >= TALK_DIST {
            *ai_state = MerchantAiState::FindingTarget;
            continue;
        }

        ship_movement.match_speed = true;

        let diff = target_vel.linvel - velocity.linvel;
        let dot = target_vel.linvel.dot(velocity.linvel);

        if diff.length() > 10.0 {
            ship_movement.movement = diff.normalize();
            ship_movement.match_speed = false;
            ship_movement.braking = false;
        } else if dot >= 30.0 {
            ship_movement.braking = true;
            ship_movement.movement = diff.normalize();
            ship_movement.match_speed = false;
        } else {
            ship_movement.match_speed = true;
            ship_movement.braking = false;
            ship_movement.movement = Vec3::ZERO;
            commands.entity(pilot.entity).insert(PilotFocused(target_ent));
        }

        if target_requested_coms || !needs_coms {
            continue;
        }

        evw_send_coms.send(RequestHailFromNpc {
            npc_ship: entity,
            player_ship: target_ent,
            ai_coms_type: AiComsType::YesNo,
        });
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
            searching_merchant_ai,
            talking_merchant_ai,
            combat_merchant_ai,
            handle_quest_npc_targetting.before(ShipMovementSet::RemoveShipMovement),
        )
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between)
            .in_set(MerchantSystemSet::MerchantAiLogic)
            .chain(),
    );
}
