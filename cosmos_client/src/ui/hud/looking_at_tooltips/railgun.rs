use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::Block,
    prelude::StructureSystems,
    registry::{Registry, identifiable::Identifiable},
    structure::systems::railgun_system::{InvalidRailgunReason, RailgunSystem},
};

use crate::ui::{
    constants,
    hud::tooltip::{GenerateLookingAtTooltipEvent, LookingAtTooltip, LookingAtTooltipSet},
    message::HudMessage,
};

const RAILGUN_MSG_LABEL: &str = "cosmos:railgun";

fn add_tooltip_text(
    mut evr_create_tooltip: EventReader<GenerateLookingAtTooltipEvent>,
    mut q_tooltip: Query<&mut LookingAtTooltip>,
    blocks: Res<Registry<Block>>,
    q_structure: Query<&StructureSystems>,
    q_railgun_system: Query<&RailgunSystem>,
    mut railgun_block_id: Local<Option<u16>>,
) {
    let railgun_block_id = match *railgun_block_id {
        Some(rg) => rg,
        None => {
            let Some(id) = blocks.from_id("cosmos:railgun_launcher") else {
                return;
            };

            *railgun_block_id = Some(id.id());

            id.id()
        }
    };

    for ev in evr_create_tooltip.read().filter(|ev| ev.block_id == railgun_block_id) {
        let Ok(ss) = q_structure.get(ev.looking_at.structure()) else {
            continue;
        };

        let Ok(rgs) = ss.query(&q_railgun_system) else {
            continue;
        };

        let Some(rg) = rgs.railguns.iter().find(|r| r.origin == ev.looking_at.coords()) else {
            continue;
        };

        let Ok(mut tooltip) = q_tooltip.get_mut(ev.tooltip_entity) else {
            continue;
        };

        if let Some(invalid) = rg.invalid_reason {
            let reason_text = match invalid {
                InvalidRailgunReason::NoMagnets => "Railgun needs more magnets",
                InvalidRailgunReason::Obstruction => "Something is obstructing the barrel",
                InvalidRailgunReason::TouchingAnother => "This railgun is sharing blocks with another railgun",
                InvalidRailgunReason::NoCapacitors => "This railgun has no capacitors to charge",
                InvalidRailgunReason::NoCooling => "This railgun has no cooling mechanism",
            };
            tooltip.add_or_modify_message(
                HudMessage::with_colored_string(format!("{} {reason_text}", constants::CROSS), css::RED.into()),
                RAILGUN_MSG_LABEL,
            );
        } else {
            tooltip.add_or_modify_message(
                HudMessage::with_colored_string(format!("{} Railgun Ready", constants::CHECK), css::GREEN.into()),
                RAILGUN_MSG_LABEL,
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, add_tooltip_text.in_set(LookingAtTooltipSet::GenerateTooltipContent));
}
