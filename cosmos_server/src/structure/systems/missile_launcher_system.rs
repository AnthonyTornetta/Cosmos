//! Server-side laser cannon logic

use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group},
    prelude::{ReadMassProperties, Velocity},
};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{data::BlockData, Block},
    entities::player::Player,
    inventory::Inventory,
    item::Item,
    logic::{logic_driver::LogicDriver, LogicInputEvent, LogicSystemSet},
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, sync::events::server_event::NettyEventWriter,
        system_sets::NetworkingSystemsSet, NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::{
        collision_handling::{CollisionBlacklist, CollisionBlacklistedEntity},
        location::{Location, LocationPhysicsSet, SetPosition},
    },
    projectiles::missile::Missile,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::{
            energy_storage_system::EnergyStorageSystem,
            laser_cannon_system::{LineSystemCooldown, SystemCooldown},
            line_system::LineBlocks,
            missile_launcher_system::{
                MissileLauncherCalculator, MissileLauncherFocus, MissileLauncherPreferredFocus, MissileLauncherProperty,
                MissileLauncherSystem, MissileSystemFailure,
            },
            StructureSystem, StructureSystems, StructureSystemsSet, SystemActive,
        },
        Structure,
    },
};

use crate::projectiles::missile::MissileTargetting;

use super::{line_system::add_line_system, sync::register_structure_system};

fn on_add_missile_launcher(mut commands: Commands, query: Query<Entity, Added<MissileLauncherSystem>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(LineSystemCooldown::default());
    }
}

fn register_missile_launcher_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<MissileLauncherProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:missile_launcher") {
        cannon.insert(block, MissileLauncherProperty { energy_per_shot: 100.0 })
    }
}

/// How fast a laser will travel (m/s) ignoring the speed of its shooter.
pub const MISSILE_BASE_VELOCITY: f32 = 20.0;

const MISSILE_SPEED_MULTIPLIER: f32 = 30.0; // higher = higher speed for way less cannons
const MISSILE_SPEED_DIVIDER: f32 = 1.0 / 5.0; // lower = more cannons required for same effect

/// How long a missile will stay alive for before despawning
pub const MISSILE_LIFETIME: Duration = Duration::from_secs(20);
/// The missile's life time may be +/- this number
pub const MISSILE_LIFETIME_FUDGE: Duration = Duration::from_secs(1);

/// How long the missile system must focus on a target before it's locked on
pub const MISSILE_FOCUS_TIME: Duration = Duration::from_secs(5);

const MAX_MISSILE_FOCUS_DISTANCE: f32 = 2000.0;

#[derive(Component, Debug)]
struct MissileTargettable;

fn add_missile_targettable(q_added_targettable: Query<Entity, Or<(Added<Structure>, Added<Player>)>>, mut commands: Commands) {
    for ent in &q_added_targettable {
        commands.entity(ent).insert(MissileTargettable);
    }
}

fn missile_lockon(
    mut q_missile_systems: Query<(&StructureSystem, &mut MissileLauncherFocus, &MissileLauncherPreferredFocus)>,
    q_structure: Query<(&Location, &GlobalTransform)>,
    q_targettable: Query<(Entity, &Location), With<MissileTargettable>>,
    time: Res<Time>,
) {
    for (structure_system, mut missile_launmcher_focus, preferred_focus) in q_missile_systems.iter_mut() {
        // Verify system is hovered
        let Ok((structure_location, g_trans)) = q_structure.get(structure_system.structure_entity()) else {
            continue;
        };

        // TODO: Make this dependent on direction the player is looking (because of camera blocks)
        let targetting_forward = g_trans.forward();

        // Find best cadidate for focusing
        let mut best_target = preferred_focus.focusing_server_entity.and_then(|ent| {
            let (ent, loc) = q_targettable.get(ent).ok()?;

            calculate_focusable_properties(ent, structure_system, loc, structure_location, targetting_forward.into())?;

            Some(ent)
        });

        if best_target.is_none() {
            best_target = q_targettable
                .iter()
                .filter_map(|(ent, loc)| {
                    let (dist, dot) =
                        calculate_focusable_properties(ent, structure_system, loc, structure_location, targetting_forward.into())?;

                    // Closer focusable targets will be somewhat preferred over distant ones.
                    Some((
                        // cast to i32 so it implements ord
                        ((dot * dist.sqrt() / MAX_MISSILE_FOCUS_DISTANCE) * MAX_MISSILE_FOCUS_DISTANCE) as i32,
                        ent,
                    ))
                })
                .min_by_key(|x| x.0)
                .map(|x| x.1);
        }

        let Some(best_target) = best_target else {
            if !matches!(*missile_launmcher_focus, MissileLauncherFocus::NotFocusing) {
                missile_launmcher_focus.clear_focus();
            }
            continue;
        };

        match missile_launmcher_focus.as_mut() {
            MissileLauncherFocus::Focusing {
                focusing_server_entity,
                focused_duration,
                complete_duration: _,
            } => {
                if *focusing_server_entity != best_target {
                    missile_launmcher_focus.change_focus(best_target, MISSILE_FOCUS_TIME);
                } else {
                    *focused_duration += Duration::from_secs_f32(time.delta_secs());
                }
            }
            MissileLauncherFocus::NotFocusing => {
                missile_launmcher_focus.change_focus(best_target, MISSILE_FOCUS_TIME);
            }
        }
    }
}

/// Returns None if this entity cannot be focused on.
///
/// Otherwise, returns Some((distance, dot))
fn calculate_focusable_properties(
    ent: Entity,
    structure_system: &StructureSystem,
    loc: &Location,
    structure_location: &Location,
    targetting_forward: Vec3,
) -> Option<(f32, f32)> {
    if ent == structure_system.structure_entity() {
        return None;
    }
    let dist = loc.distance_sqrd(structure_location);
    if dist > MAX_MISSILE_FOCUS_DISTANCE * MAX_MISSILE_FOCUS_DISTANCE {
        return None;
    }
    let direction = (*loc - *structure_location).absolute_coords_f32().normalize_or_zero();
    let dot = targetting_forward.dot(direction);
    if dot < 0.9 {
        return None;
    };

    Some((dist, dot))
}

fn update_missile_system(
    mut query: Query<(
        &MissileLauncherSystem,
        &MissileLauncherFocus,
        &StructureSystem,
        &mut LineSystemCooldown,
        Has<SystemActive>,
    )>,
    mut es_query: Query<&mut EnergyStorageSystem>,
    q_systems: Query<(
        Entity,
        &StructureSystems,
        &Structure,
        &Location,
        &GlobalTransform,
        &Velocity,
        Option<&Pilot>,
    )>,
    q_player: Query<&Player>,
    time: Res<Time>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut q_inventory: Query<(&mut Inventory, &BlockData)>,
    items: Res<Registry<Item>>,
    mut nevw_system_failure: NettyEventWriter<MissileSystemFailure>,
) {
    for (missile_launcher_system, focus, system, mut cooldown, system_active) in query.iter_mut() {
        let Ok((ship_entity, systems, structure, location, global_transform, ship_velocity, pilot)) =
            q_systems.get(system.structure_entity())
        else {
            continue;
        };
        let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) else {
            continue;
        };

        let sec = time.elapsed_secs();

        let mut any_fired = false;

        let default_cooldown = SystemCooldown {
            cooldown_time: Duration::from_secs(5),
            ..Default::default()
        };

        cooldown.remove_unused_cooldowns(missile_launcher_system);

        let Some(missile_item) = items.from_id("cosmos:missile") else {
            error!("No cosmos:missile item!");
            continue;
        };

        for line in missile_launcher_system.lines.iter() {
            let cooldown = cooldown.lines.entry(line.start).or_insert(default_cooldown);

            if sec - cooldown.last_use_time <= cooldown.cooldown_time.as_secs_f32() {
                continue;
            }

            if !((system_active || line.active()) && energy_storage_system.get_energy() >= line.property.energy_per_shot) {
                continue;
            }

            if !q_inventory
                .iter_mut()
                .filter(|(_, bd)| bd.identifier.block.structure() == system.structure_entity())
                .map(|x| x.0)
                .any(|mut inv| inv.take_and_remove_item(missile_item, 1, &mut commands).0 == 0)
            {
                if let Some(pilot) = pilot {
                    if let Ok(player) = q_player.get(pilot.entity) {
                        nevw_system_failure.send(MissileSystemFailure::NoAmmo, player.id());
                    }
                }
                break;
            }

            cooldown.last_use_time = sec;
            any_fired = true;
            energy_storage_system.decrease_energy(line.property.energy_per_shot);

            let location = structure.block_world_location(line.start, global_transform, location);

            let relative_direction = line.direction.as_vec3();

            let missile_vel = MISSILE_BASE_VELOCITY + (line.len as f32 * MISSILE_SPEED_DIVIDER + 1.0).ln() * MISSILE_SPEED_MULTIPLIER;

            let missile_velocity = global_transform.affine().matrix3.mul_vec3(relative_direction) * missile_vel;

            // TODO: Make missile launcher take item and strength is determined by the item they hold
            let strength = 10.0; //(5.0 * line.len as f32).powf(1.2);

            let lifetime = Duration::from_secs_f32(
                MISSILE_LIFETIME.as_secs_f32() + (MISSILE_LIFETIME_FUDGE.as_secs_f32() * (rand::random::<f32>() - 0.5) * 2.0),
            );

            let mut missile_cmds = commands.spawn((
                Missile {
                    color: line.color,
                    strength,
                    lifetime,
                },
                Transform::from_xyz(0.0, 0.0, 0.0).looking_at(missile_velocity, Vec3::Y),
                location,
                SetPosition::Transform,
                Velocity {
                    linvel: missile_velocity + ship_velocity.linvel,
                    ..Default::default()
                },
                LoadingDistance::new(1, 2),
                CollisionGroups::new(Group::ALL, Group::ALL),
                CollisionBlacklist::single(CollisionBlacklistedEntity {
                    entity: system.structure_entity(),
                    search_parents: true,
                }),
                ReadMassProperties::default(),
            ));

            if let Some(targetting) = focus.locked_on_to() {
                missile_cmds.insert(MissileTargetting {
                    targetting,
                    targetting_fudge: Vec3::new(rand::random(), rand::random(), rand::random()) * 4.0,
                });
            }
        }

        if any_fired {
            server.broadcast_message(
                NettyChannelServer::StructureSystems,
                cosmos_encoder::serialize(&ServerStructureSystemMessages::MissileLauncherSystemFired { ship_entity }),
            );
        }
    }
}

fn missile_launcher_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<(&Structure, &StructureSystems)>,
    mut q_missile_launcher_system: Query<&mut MissileLauncherSystem>,
) {
    for ev in evr_logic_input.read() {
        let Ok((structure, systems)) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:missile_launcher" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let Ok(mut missile_launcher_system) = systems.query_mut(&mut q_missile_launcher_system) else {
            continue;
        };
        let Some(line) = missile_launcher_system.mut_line_containing(ev.block.coords()) else {
            continue;
        };

        let active = logic_driver
            .read_all_inputs(ev.block.coords(), structure.block_rotation(ev.block.coords()))
            .iter()
            .any(|signal| *signal != 0);

        if active {
            line.mark_block_active(ev.block.coords());
        } else {
            line.mark_block_inactive(ev.block.coords());
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_line_system::<MissileLauncherProperty, MissileLauncherCalculator>(app);

    app.add_systems(
        Update,
        update_missile_system
            .run_if(in_state(GameState::Playing))
            .after(LocationPhysicsSet::DoPhysics)
            .before(NetworkingSystemsSet::SyncComponents),
    )
    .add_systems(OnEnter(GameState::PostLoading), register_missile_launcher_blocks)
    .add_systems(
        Update,
        (add_missile_targettable, on_add_missile_launcher, missile_lockon)
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    )
    .add_systems(
        Update,
        missile_launcher_input_event_listener
            .in_set(LogicSystemSet::Consume)
            .in_set(StructureSystemsSet::UpdateSystems)
            .ambiguous_with(LogicSystemSet::Consume),
    );

    register_structure_system::<MissileLauncherSystem>(app, true, "cosmos:missile_launcher");
}
