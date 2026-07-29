use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_rotation::BlockRotation},
    physics::location::Location,
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        full_structure::FullStructure,
        loading::StructureLoadingSet,
        station::Station,
    },
};

use crate::structure::station::loading::StationNeedsCreated;

#[derive(Debug, Reflect, Component)]
pub struct WarpGateNeedsCreated {
    pub destination: Location,
}

fn circle(
    s: &mut Structure,
    origin: BlockCoordinate,
    radius: CoordinateType,
    block: &Block,
    br: BlockRotation,
    blocks: &Registry<Block>,
    x: BlockCoordinate,
    y: BlockCoordinate,
    thickness: f32,
) {
    assert!(x.x + x.y + x.z == 1);
    assert!(y.x + y.y + y.z == 1);

    let r = radius as UnboundCoordinateType;
    let x = UnboundBlockCoordinate::from(x);
    let y = UnboundBlockCoordinate::from(y);

    for dy in -r..=r {
        for dx in -r..=r {
            let offset = x * dx + y * dy;

            let Ok(coord) = BlockCoordinate::try_from(offset + origin) else {
                continue;
            };
            let v = Vec3::new(offset.x as f32, offset.y as f32, offset.z as f32);

            let diff = radius as f32 - v.length();
            if diff >= 0.0 && diff <= thickness {
                s.set_block_at(coord, block, br, blocks, None);
            }
        }
    }
}

fn on_needs_warpgate_generated(mut commands: Commands, q_needs_wg: Query<(Entity, &WarpGateNeedsCreated)>, blocks: Res<Registry<Block>>) {
    for (ent, needs_made) in q_needs_wg.iter() {
        let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(20, 20, 20)));

        let mut origin = structure.block_dimensions();
        origin.x /= 2;
        origin.y /= 2;
        origin.z /= 2;

        let Some(frame) = blocks.from_id("cosmos:warp_gate_frame") else {
            error!("Missing warp gate frame!");
            return;
        };

        let Some(portal) = blocks.from_id("cosmos:portal") else {
            error!("Missing warp gate frame!");
            return;
        };

        let radius = (origin.x as f32 * 0.95) as CoordinateType;

        circle(
            &mut structure,
            origin,
            radius - 3,
            portal,
            Default::default(),
            &blocks,
            BlockCoordinate::X,
            BlockCoordinate::Y,
            radius as f32,
        );

        circle(
            &mut structure,
            origin,
            radius,
            frame,
            Default::default(),
            &blocks,
            BlockCoordinate::X,
            BlockCoordinate::Y,
            7.0,
        );

        circle(
            &mut structure,
            BlockCoordinate::try_from(origin - BlockCoordinate::Z).unwrap(),
            radius - 3,
            frame,
            Default::default(),
            &blocks,
            BlockCoordinate::X,
            BlockCoordinate::Y,
            4.0,
        );

        circle(
            &mut structure,
            origin + BlockCoordinate::Z,
            radius - 3,
            frame,
            Default::default(),
            &blocks,
            BlockCoordinate::X,
            BlockCoordinate::Y,
            4.0,
        );

        commands
            .entity(ent)
            .insert((
                StationNeedsCreated { already_has_core: true },
                Station,
                // ev.station_location,
                structure,
                // Transform::from_rotation(ev.rotation),
            ))
            .remove::<WarpGateNeedsCreated>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        on_needs_warpgate_generated
            .in_set(StructureLoadingSet::LoadStructure)
            .run_if(in_state(GameState::Playing)),
    );
}
