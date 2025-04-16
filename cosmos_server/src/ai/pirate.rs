use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Or, With, Without},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::error,
    prelude::{Has, in_state},
};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    events::structure::StructureEventListenerSet,
    faction::{FactionId, Factions},
    netty::system_sets::NetworkingSystemsSet,
    physics::location::Location,
    projectiles::missile::Missile,
    state::GameState,
    structure::{
        StructureTypeSet,
        shared::{DespawnWithStructure, MeltingDown},
        ship::{Ship, pilot::Pilot, ship_movement::ShipMovementSet},
    },
};

use crate::{
    persistence::{
        SerializedData,
        loading::{LOADING_SCHEDULE, LoadingSystemSet, NeedsLoaded},
        saving::{SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::systems::thruster_system::MaxShipSpeedModifier,
    universe::spawners::pirate::Pirate,
};

use super::{
    AiControlled,
    combat::{AiTargetting, CombatAi, CombatAiSystemSet},
    hit_tracking::DifficultyIncreaseOnKill,
};

#[derive(Component)]
pub struct PirateTarget;

const PIRATE_MAX_CHASE_DISTANCE: f32 = 20_000.0;

/// Attempt to maintain a distance of ~500 blocks from closest target
fn handle_pirate_targetting(
    mut commands: Commands,
    mut q_pirates: Query<
        (Entity, &Location),
        (With<Pirate>, Without<Missile>, With<AiControlled>), // Without<Missile> fixes ambiguity issues
    >,
    q_targets: Query<(Entity, &Location, Has<MeltingDown>), (Without<Pirate>, With<PirateTarget>)>,
) {
    for (pirate_ent, pirate_loc) in q_pirates.iter_mut() {
        let Some((target_ent, _, _)) = q_targets
            .iter()
            .filter(|x| x.1.is_within_reasonable_range(pirate_loc))
            // add a large penalty for something that's melting down so they prioritize non-melting down things
            .min_by_key(|(_, this_loc, melting_down)| {
                // Makes it only target melting down targets if they're the only one nearby
                let melting_down_punishment = if *melting_down { 100_000_000_000_000 } else { 0 };

                this_loc.distance_sqrd(pirate_loc).floor() as u64 + melting_down_punishment
            })
        else {
            continue;
        };

        commands.entity(pirate_ent).insert(AiTargetting(target_ent));
    }
}

fn add_pirate_targets(
    mut commands: Commands,
    q_should_be_targets: Query<Entity, (Without<PirateTarget>, Or<(With<Player>, (With<Ship>, Without<Pirate>))>)>,
) {
    for ent in &q_should_be_targets {
        commands.entity(ent).insert(PirateTarget);
    }
}

fn add_pirate_ai(mut commands: Commands, q_needs_ai: Query<Entity, (With<Pirate>, Without<CombatAi>)>) {
    for ent in &q_needs_ai {
        let pilot_ent = commands
            .spawn((
                Name::new("Fake pirate pilot"),
                PiratePilot,
                DespawnWithStructure,
                Pilot { entity: ent },
            ))
            .id();

        let mut ai = CombatAi {
            max_chase_distance: PIRATE_MAX_CHASE_DISTANCE,
            ..Default::default()
        };
        ai.randomize_inaccuracy();

        commands
            .entity(ent)
            .insert((AiControlled, ai, MaxShipSpeedModifier(0.8), Pilot { entity: pilot_ent }))
            .add_child(pilot_ent);
    }
}

fn on_melt_down(
    q_is_pirate: Query<(), With<PiratePilot>>,
    q_melting_down: Query<(Entity, Option<&Pilot>), (With<MeltingDown>, With<CombatAi>, With<AiControlled>)>,
    mut commands: Commands,
) {
    for (ent, pilot) in &q_melting_down {
        commands.entity(ent).remove::<(CombatAi, AiControlled, Pirate, Pilot)>();

        if let Some(pilot) = pilot {
            if q_is_pirate.contains(pilot.entity) {
                commands.entity(pilot.entity).insert(NeedsDespawned);
            }
        }
    }
}

#[derive(Component)]
struct PiratePilot;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum PirateSystemSet {
    PirateAiLogic,
}

fn on_save_pirate(mut q_pirate: Query<&mut SerializedData, With<Pirate>>) {
    for mut serialized_data in q_pirate.iter_mut() {
        serialized_data.serialize_data("cosmos:pirate", &true);
    }
}

fn on_load_pirate(mut commands: Commands, query: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    for (entity, serialized_data) in query.iter() {
        if serialized_data.deserialize_data::<bool>("cosmos:pirate").unwrap_or(false) {
            commands.entity(entity).insert(Pirate);
        }
    }
}

fn apply_pirate_faction(factions: Res<Factions>, mut commands: Commands, q_pirate: Query<Entity, (Without<FactionId>, With<Pirate>)>) {
    for ent in q_pirate.iter() {
        let Some(pirate_faction) = factions.from_name("Pirate") else {
            error!("No pirate faction found! Cannot assign pirate to faction.");
            return;
        };

        commands.entity(ent).insert(*pirate_faction.id());
    }
}

/// TODO: Load this from config
///
/// How much killing a pirate will increase the difficulty.
/// Aka, if you do 100% of the damage, your strength percentage will increase by this percent.
const PIRATE_DIFFICULTY_INCREASE: f32 = 5.0;

fn add_difficuly_increase(mut commands: Commands, q_merchant: Query<Entity, (With<Pirate>, Without<DifficultyIncreaseOnKill>)>) {
    for ent in &q_merchant {
        commands.entity(ent).insert(DifficultyIncreaseOnKill(PIRATE_DIFFICULTY_INCREASE));
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        PirateSystemSet::PirateAiLogic
            .before(CombatAiSystemSet::CombatAiLogic)
            .in_set(StructureTypeSet::Ship)
            .after(LoadingSystemSet::DoneLoading)
            .after(StructureEventListenerSet::ChangePilotListener),
    )
    .add_systems(
        Update,
        (
            on_melt_down,
            add_pirate_ai,
            add_difficuly_increase,
            apply_pirate_faction,
            add_pirate_targets,
            handle_pirate_targetting.before(ShipMovementSet::RemoveShipMovement),
        )
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between)
            .in_set(PirateSystemSet::PirateAiLogic)
            .chain(),
    )
    .add_systems(LOADING_SCHEDULE, on_load_pirate.in_set(LoadingSystemSet::DoLoading))
    .add_systems(SAVING_SCHEDULE, on_save_pirate.in_set(SavingSystemSet::DoSaving));
}
