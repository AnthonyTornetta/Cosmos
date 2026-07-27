use bevy::prelude::*;
use bevy_rapier3d::dynamics::{RigidBody, Velocity};
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockMessagesSet, BlockPlaceMessage},
        block_face::BlockFace,
    },
    events::cancellable::{Cancellable, CancellableMessage},
    physics::location::{Location, SetPosition},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        coordinates::{BlockCoordinate, ChunkCoordinate},
        full_structure::FullStructure,
        ship::Ship,
        systems::dock_system::Docked,
    },
};

use crate::structure::{ship::loading::ShipNeedsCreated, systems::DockedEntities};

fn handle_placing_dock(
    mut evr_place: MessageMutator<Cancellable<BlockPlaceMessage>>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<(&Structure, &GlobalTransform)>,
    mut commands: Commands,
    mut dock_blocks: Local<Option<Vec<u16>>>,
    q_docked_ents: Query<&DockedEntities>,
    q_docked: Query<&Docked>,
) {
    for place_event in evr_place.read() {
        let Cancellable::Active(place_event_data) = place_event else {
            continue;
        };

        if dock_blocks.is_none() {
            *dock_blocks = Some(
                vec![blocks.from_id("cosmos:dock"), blocks.from_id("cosmos:pan_dock")]
                    .into_iter()
                    .flatten()
                    .map(|x| x.id())
                    .collect::<Vec<_>>(),
            );
        }

        let dock_blocks = dock_blocks.as_ref().unwrap();

        if !dock_blocks.contains(&place_event_data.block_id) {
            continue;
        }

        let Ok((structure, structure_g_trans)) = q_structure.get(place_event_data.block.structure()) else {
            continue;
        };

        let mut coords = None;

        info!("Place coord: {}", place_event_data.block.coords());

        let same_front_direction = BlockCoordinate::try_from(
            place_event_data.block_rotation.direction_of(BlockFace::Back).to_coordinates() + place_event_data.block.coords(),
        )
        .map(|below_coord| {
            info!("Below coord: {below_coord}"); // correct
            if !structure.is_within_blocks(below_coord) || !dock_blocks.contains(&structure.block_id_at(below_coord)) {
                info!("not within ;(");
                return false;
            }

            coords = Some(below_coord);

            structure.block_rotation(below_coord).direction_of(BlockFace::Front)
                == place_event_data.block_rotation.direction_of(BlockFace::Front)
        })
        .unwrap_or(false);

        // let facing_front_direction = BlockCoordinate::try_from(
        //     place_event_data.block_rotation.direction_of(BlockFace::Front).to_coordinates() + place_event_data.block.coords(),
        // )
        // .map(|above_coord| {
        //     if !structure.is_within_blocks(above_coord) || !dock_blocks.contains(&structure.block_id_at(above_coord)) {
        //         return false;
        //     }
        //
        //     // This coordinate tasks presidence if possible
        //     coords = Some(above_coord);
        //
        //     structure.block_rotation(above_coord).direction_of(BlockFace::Front)
        //         == place_event_data.block_rotation.direction_of(BlockFace::Front)
        // })
        // .unwrap_or(false);

        if !(same_front_direction/*|| facing_front_direction*/) {
            return;
        }

        let Some(placed_on_coords) = coords else {
            return;
        };

        // first verify there already another ship docked here so no silliness ensues
        if let Ok(docked) = q_docked_ents.get(place_event_data.block.structure()) {
            if docked.iter().any(|ent| {
                let Ok(d) = q_docked.get(ent) else {
                    return false;
                };

                d.to_block == place_event_data.block.coords()
            }) {
                // already a docked ent there
                continue;
            }
        }

        info!("A dock was placed on another dock! Create a pre-docked structure!");
        let place_event_data = (*place_event_data).clone();
        place_event.cancel();

        let mut turret_structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(4, 4, 4)));
        let ship = Ship::new_for_structure(&turret_structure);
        let default_core_coords = Ship::default_ship_core_coords(&turret_structure);

        info!("PO C: {:?}", structure.block_rotation(placed_on_coords));
        info!("PO C INV: {:?}", structure.block_rotation(placed_on_coords).inverse());

        turret_structure.set_block_at(
            default_core_coords,
            blocks.from_numeric_id(place_event_data.block_id),
            structure.block_rotation(placed_on_coords).inverse(),
            &blocks,
            None,
        );

        let relative_translation = structure.block_relative_position(place_event_data.block.coords());

        let parent_anchor = structure.block_relative_position(placed_on_coords)
            + structure.block_rotation(placed_on_coords).direction_of(BlockFace::Front).as_vec3() * 0.5;

        let child_anchor = turret_structure.block_relative_position(default_core_coords)
            + turret_structure
                .block_rotation(default_core_coords)
                .direction_of(BlockFace::Front)
                .as_vec3()
                * 0.5;

        info!("Parent anchor: {parent_anchor}");
        info!("Child anchor: {child_anchor}");

        commands.spawn((
            Name::new("Turret ship"),
            Velocity::default(),
            ship,
            ShipNeedsCreated { already_has_core: true },
            Transform::from_rotation(structure_g_trans.rotation()),
            Location::default(),
            SetPosition::RelativeTo {
                entity: place_event_data.block.structure(),
                offset: relative_translation,
            },
            turret_structure,
            RigidBody::Dynamic,
            Docked {
                to: place_event_data.block.structure(),
                to_block: placed_on_coords,
                this_block: default_core_coords,
                relative_rotation: Quat::IDENTITY,
                relative_translation,
                rotate_x: false,
                rotate_y: true,
                rotate_z: false,
                parent_anchor,
                child_anchor,
            },
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_placing_dock
            .in_set(BlockMessagesSet::ProcessMessagesPrePlacement)
            .run_if(in_state(GameState::Playing)),
    );
}
