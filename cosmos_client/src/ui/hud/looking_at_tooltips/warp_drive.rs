use bevy::{color::palettes::css, prelude::*};
use bevy_rapier3d::prelude::ReadMassProperties;
use cosmos_core::{
    block::Block,
    prelude::StructureSystems,
    registry::{Registry, identifiable::Identifiable},
    structure::systems::warp::warp_drive::{WarpDriveSystem, WarpDriveSystemState},
};

use crate::ui::{
    constants,
    hud::tooltip::{GenerateLookingAtTooltipEvent, LookingAtTooltip, LookingAtTooltipSet},
    message::HudMessage,
};

const WARP_DRIVE_MSG_LABEL: &str = "cosmos:warp_drive";

fn add_tooltip_text(
    mut evr_create_tooltip: EventReader<GenerateLookingAtTooltipEvent>,
    mut q_tooltip: Query<&mut LookingAtTooltip>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<(&StructureSystems, &ReadMassProperties)>,
    q_warp_system: Query<&WarpDriveSystem>,
    mut warp_drive_block_id: Local<Option<u16>>,
) {
    let warp_drive_block_id = match *warp_drive_block_id {
        Some(rg) => rg,
        None => {
            let Some(id) = blocks.from_id("cosmos:warp_drive") else {
                return;
            };

            *warp_drive_block_id = Some(id.id());

            id.id()
        }
    };

    for ev in evr_create_tooltip.read().filter(|ev| ev.block_id == warp_drive_block_id) {
        let Ok((ss, mass)) = q_structure.get(ev.looking_at.structure()) else {
            continue;
        };

        let Ok(warp_system) = ss.query(&q_warp_system) else {
            continue;
        };

        let Ok(mut tooltip) = q_tooltip.get_mut(ev.tooltip_entity) else {
            continue;
        };

        match warp_system.compute_state(mass.get().mass) {
            WarpDriveSystemState::ReadyToWarp => {
                tooltip.add_or_modify_message(
                    HudMessage::with_colored_string(format!("{} Ready to Jump", constants::CHECK), css::GREEN.into()),
                    WARP_DRIVE_MSG_LABEL,
                );
            }
            WarpDriveSystemState::Charging => {
                tooltip.add_or_modify_message(
                    HudMessage::with_colored_string("Warp Drive Charging", css::YELLOW.into()),
                    WARP_DRIVE_MSG_LABEL,
                );
            }
            WarpDriveSystemState::StructureTooBig => {
                tooltip.add_or_modify_message(
                    HudMessage::with_colored_string(
                        format!("{} Structure too Massive! Place more warp drives", constants::CROSS),
                        css::RED.into(),
                    ),
                    WARP_DRIVE_MSG_LABEL,
                );
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, add_tooltip_text.in_set(LookingAtTooltipSet::GenerateTooltipContent));
}
