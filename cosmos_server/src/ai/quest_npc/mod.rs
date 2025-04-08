//! The merchant NPC's AI logic

use std::num::NonZeroU32;

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
    faction::{Faction, FactionId, FactionRelation, Factions},
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::Ship,
    projectiles::missile::Missile,
    quest::OngoingQuestDetails,
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
    coms::{NpcSendComsMessage, RequestHailFromNpc},
    persistence::{
        loading::LoadingSystemSet,
        make_persistent::{make_persistent, DefaultPersistentComponent},
    },
    quest::AddQuestEvent,
    structure::systems::thruster_system::MaxShipSpeedModifier,
};

use super::{
    combat::{AiTargetting, CombatAi, CombatAiSystemSet},
    hit_tracking::{DifficultyIncreaseOnKill, Hitters},
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
    q_targets: Query<(Entity, &EntityId, &Location, Has<MeltingDown>, Option<&FactionId>)>,
    factions: Res<Factions>,
) {
    for (merchant_ent, merchant_loc, my_faction_id, mut ai_state) in q_merchants.iter_mut() {
        let Some(quest_npc_faction) = factions.from_id(my_faction_id) else {
            warn!("Quest NPC faction not found!");
            continue;
        };

        let Some((target_ent, _, _, _, _)) = q_targets
            .iter()
            .filter(|x| {
                x.2.is_within_reasonable_range(merchant_loc)
                    && x.2.distance_sqrd(merchant_loc) < QUEST_NPC_MAX_CHASE_DISTANCE * QUEST_NPC_MAX_CHASE_DISTANCE
            })
            .filter(|(_, entity_id, _, _, faction_id)| {
                quest_npc_faction.relation_with_entity(entity_id, faction_id.and_then(|id| factions.from_id(id))) == FactionRelation::Enemy
            })
            // add a large penalty for something that's melting down so they prioritize non-melting down things
            .min_by_key(|(_, _, this_loc, melting_down, _)| {
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
    q_needs_ai: Query<
        Entity,
        (
            With<MerchantFederation>,
            Or<(Without<SaidNoList>, Without<Hitters>, Without<MerchantAiState>)>,
        ),
    >,
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
                Hitters::default(),
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
    Fleeing,
}

const TALK_DIST: f32 = 1000.0;

fn any_war_targets(
    m_loc: &Location,
    m_fac: &Faction,
    factions: &Factions,
    q_war_targets: &Query<(&EntityId, &Location, Option<&FactionId>)>,
) -> bool {
    q_war_targets.iter().any(|x| {
        x.1.is_within_reasonable_range(m_loc)
            && x.1.distance_sqrd(m_loc) < QUEST_NPC_MAX_CHASE_DISTANCE * QUEST_NPC_MAX_CHASE_DISTANCE
            && m_fac.relation_with_entity(x.0, x.2.and_then(|x| factions.from_id(x))) == FactionRelation::Enemy
    })
}

fn fleeing_merchant_ai(
    mut q_merchant: Query<
        (&Location, &mut Transform, &FactionId, &mut MerchantAiState, &mut ShipMovement),
        (With<MerchantFederation>, With<AiControlled>),
    >,
    q_war_targets: Query<(&EntityId, &Location, Option<&FactionId>)>,
    q_targets: Query<(&EntityId, &Location, Option<&FactionId>, &Pilot), (With<Ship>, Without<AiControlled>)>,
    factions: Res<Factions>,
) {
    for (m_loc, mut m_trans, m_fac, mut ai_state, mut ship_movement) in q_merchant.iter_mut() {
        if !matches!(*ai_state, MerchantAiState::Fleeing) {
            continue;
        }

        let Some(m_fac) = factions.from_id(m_fac) else {
            warn!("Merchant faction not found!");
            continue;
        };

        if any_war_targets(m_loc, m_fac, &factions, &q_war_targets) {
            *ai_state = MerchantAiState::Fighting;
            continue;
        }

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

        // fly in the opposite direction of its closest target
        m_trans.look_to(-(target - *m_loc).absolute_coords_f32(), Vec3::Y);
        ship_movement.movement = Vec3::Z;
        ship_movement.braking = false;
    }
}

fn searching_merchant_ai(
    mut q_merchant: Query<
        (&Location, &mut Transform, &FactionId, &mut MerchantAiState, &mut ShipMovement),
        (With<MerchantFederation>, With<AiControlled>),
    >,
    q_war_targets: Query<(&EntityId, &Location, Option<&FactionId>)>,
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

        if any_war_targets(m_loc, m_fac, &factions, &q_war_targets) {
            *ai_state = MerchantAiState::Fighting;
            continue;
        }

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
        ship_movement.braking = false;
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

fn on_change_coms(
    mut evw_send_coms: EventWriter<NpcSendComsMessage>,
    q_create_coms: Query<(&Parent, &ComsChannel), Changed<ComsChannel>>,
    factions: Res<Factions>,
    q_faction: Query<&FactionId>,
    q_entity_id: Query<&EntityId>,
    q_pilot: Query<&Pilot>,
    mut q_merchant: Query<&mut MerchantAiState, With<MerchantFederation>>,
    mut evw_start_quest: EventWriter<AddQuestEvent>,
) {
    enum ComsState {
        Intro,
        Accepted,
        OnQuest,
        SaidNo,
    }

    for (parent, coms) in q_create_coms.iter() {
        let Ok(mut merchant_ai_state) = q_merchant.get_mut(parent.get()) else {
            continue;
        };

        if let Some(last) = coms.messages.last() {
            if last.sender == parent.get() {
                // Don't reply to ourselves
                continue;
            }
        }

        if let Ok(pilot) = q_pilot.get(coms.with) {
            if let Some(faction) = q_faction.get(parent.get()).ok().and_then(|x| factions.from_id(x)) {
                let with_fac = q_faction.get(pilot.entity).ok().and_then(|x| factions.from_id(x));
                if let Ok(with_ent_id) = q_entity_id.get(pilot.entity) {
                    if faction.relation_with_entity(with_ent_id, with_fac) == FactionRelation::Enemy {
                        evw_send_coms.send(NpcSendComsMessage {
                            message: "BEGONE SCALLYWAG!!!".to_owned(),
                            from_ship: parent.get(),
                            to_ship: coms.with,
                        });
                        continue;
                    }
                }
            }
        }

        let mut itr = coms.messages.iter();

        let (intro, response, next) = (itr.next(), itr.next().map(|x| x.text.as_str()), itr.next());

        let state = match (intro, response, next) {
            (None, None, None) => ComsState::Intro,
            (Some(_), Some("Yes"), None) => ComsState::Accepted,
            (Some(_), Some("No"), _) => ComsState::SaidNo,
            _ => ComsState::OnQuest,
        };

        let response = match state {
            ComsState::SaidNo => "I understand. I, too, value my life.",
            ComsState::Accepted => {
                if let Ok(pilot) = q_pilot.get(coms.with) {
                    evw_start_quest.send(AddQuestEvent {
                        unlocalized_name: "cosmos:fight_pirate".into(),
                        to: pilot.entity,
                        details: OngoingQuestDetails {
                            payout: Some(NonZeroU32::new(500_000).unwrap()),
                            location: None,
                        },
                    });
                }

                "Thank you brave warrior! Show them no mercy!"
            }
            ComsState::OnQuest => "You're already on a quest",
            ComsState::Intro => "Please help us! A sector not far from here has been overtaken by pirates! Should you lend us your aid, you will be rewarded handsomely. Would you please lend us your aid by killing these dastardly foes?",
        }
        .to_owned();

        if matches!(*merchant_ai_state, MerchantAiState::Talking) {
            *merchant_ai_state = MerchantAiState::Fleeing;
        }

        evw_send_coms.send(NpcSendComsMessage {
            message: response,
            from_ship: parent.get(),
            to_ship: coms.with,
        });
    }
}

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
            &mut Transform,
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
    q_war_targets: Query<(&EntityId, &Location, Option<&FactionId>)>,
) {
    for (entity, m_loc, m_fac, mut ai_state, mut ship_movement, said_no_list, velocity, mut transform) in q_merchant.iter_mut() {
        if !matches!(*ai_state, MerchantAiState::Talking) {
            continue;
        }

        let Some(m_fac) = factions.from_id(m_fac) else {
            warn!("Merchant faction not found!");
            continue;
        };

        if any_war_targets(m_loc, m_fac, &factions, &q_war_targets) {
            *ai_state = MerchantAiState::Fighting;
            continue;
        }

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

        let Some((target_ent, _, target_loc, _, _, target_vel, target_requested_coms)) = target else {
            continue;
        };

        let dist = target_loc.distance_sqrd(m_loc).sqrt();
        if dist >= TALK_DIST {
            *ai_state = MerchantAiState::FindingTarget;
            continue;
        }

        let diff = target_vel.linvel - velocity.linvel;
        let diff_len = diff.length();

        let should_brake = (target_vel.linvel - (velocity.linvel * 0.9)).length() < diff_len;

        if diff_len > 30.0 && should_brake {
            ship_movement.braking = true;
            ship_movement.movement = Vec3::ZERO;
            ship_movement.match_speed = false;
        } else if diff_len > 10.0 {
            transform.rotation = transform
                .rotation
                .lerp(transform.looking_to(diff.normalize(), Vec3::Y).rotation, 0.3);
            ship_movement.movement = Vec3::Z;
            ship_movement.match_speed = false;
            ship_movement.braking = false;
        } else {
            ship_movement.match_speed = true;
            ship_movement.braking = false;
            ship_movement.movement = Vec3::ZERO;
            commands.entity(entity).insert(PilotFocused(target_ent));
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
            fleeing_merchant_ai,
            combat_merchant_ai,
            handle_quest_npc_targetting.before(ShipMovementSet::RemoveShipMovement),
            on_change_coms,
        )
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between)
            .in_set(MerchantSystemSet::MerchantAiLogic)
            .chain(),
    )
    .register_type::<MerchantAiState>();
}
