//! Represents all the blocks present in the game.
//!
//! This list is dynamic, and may grow & shrink at any time.
//!
//! The only guarenteed block is air ("cosmos:air").

use crate::block::block_builder::BlockBuilder;
use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::color::Srgba;
use bevy::prelude::{App, EventWriter, OnEnter, ResMut, States};

use super::{Block, BlockProperty};

pub mod fluid;

/// Air's ID - this block will always exist
pub const AIR_BLOCK_ID: u16 = 0;

/// When creating a block with color variations, it should be these specific colors. The id should
/// be `block_name_{color}`.
pub const COLORS: [&str; 17] = [
    "red",
    "orange",
    "yellow",
    "green",
    "mint",
    "cyan",
    "aqua",
    "blue",
    "purple",
    "magenta",
    "pink",
    "brown",
    "black",
    "dark_grey",
    "grey",
    "light_grey",
    "white",
];

/// The values linked to [`COLORS`]. The orders match each other.
#[rustfmt::skip]
#[expect(clippy::eq_op)]
pub const COLOR_VALUES: [Srgba; 17] = [
    Srgba { red: 255.0 / 255.0, green: 0.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 255.0 / 255.0, green: 165.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 255.0 / 255.0, green: 255.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 0.0 / 255.0, green: 255.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 62.0 / 255.0, green: 180.0 / 255.0, blue: 137.0 / 255.0, alpha: 1.0 },
    Srgba { red: 0.0 / 255.0, green: 183.0 / 255.0, blue: 235.0 / 255.0, alpha: 1.0 },
    Srgba { red: 0.0 / 255.0, green: 255.0 / 255.0, blue: 255.0 / 255.0, alpha: 1.0 },
    Srgba { red: 0.0 / 255.0, green: 0.0 / 255.0, blue: 255.0 / 255.0, alpha: 1.0 },
    Srgba { red: 160.0 / 255.0, green: 32.0 / 255.0, blue: 240.0 / 255.0, alpha: 1.0 },
    Srgba { red: 255.0 / 255.0, green: 0.0 / 255.0, blue: 255.0 / 255.0, alpha: 1.0 },
    Srgba { red: 255.0 / 255.0, green: 192.0 / 255.0, blue: 203.0 / 255.0, alpha: 1.0 },
    Srgba { red: 150.0 / 255.0, green: 75.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 0.0 / 255.0, green: 0.0 / 255.0, blue: 0.0 / 255.0, alpha: 1.0 },
    Srgba { red: 82.0 / 255.0, green: 82.0 / 255.0, blue: 82.0 / 255.0, alpha: 1.0 },
    Srgba { red: 128.0 / 255.0, green: 128.0 / 255.0, blue: 128.0 / 255.0, alpha: 1.0 },
    Srgba { red: 174.0 / 255.0, green: 174.0 / 255.0, blue: 174.0 / 255.0, alpha: 1.0 },
    Srgba { red: 255.0 / 255.0, green: 255.0 / 255.0, blue: 255.0 / 255.0, alpha: 1.0 },
];

fn add_cosmos_blocks(
    mut blocks: ResMut<Registry<Block>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    blocks.register(
        BlockBuilder::new("cosmos:stone", 10.0, 50.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:grass", 3.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:dirt", 3.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cherry_leaf", 0.1, 1.0, 1.0)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_log", 3.0, 30.0, 7.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:redwood_leaf", 0.1, 1.0, 1.0)
            .add_property(BlockProperty::Transparent)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_core", 2.0, 20.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energy_cell", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:stores_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:passive_generator", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:produces_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:laser_cannon", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .add_connection_group("cosmos:uses_logic")
            .add_connection_group("cosmos:consumes_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_hull_dark_grey", 4.0, 100.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:thruster", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:consumes_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:light_white", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:glass", 4.0, 100.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:glass")
            .connect_to_group("cosmos:glass")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ice".to_owned(), 1.9, 40.0, 10.0)
            .add_connection_group("cosmos:ice")
            .connect_to_group("cosmos:ice")
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:molten_stone", 10.0, 50.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:water".to_owned(), 2.0, 50.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Fluid)
            .add_connection_group("cosmos:fluid")
            .connect_to_group("cosmos:fluid")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:lava", 10.0, 50.0, 10.0)
            .add_property(BlockProperty::Fluid)
            .add_connection_group("cosmos:fluid")
            .connect_to_group("cosmos:fluid")
            .create(),
    );

    blocks.register(BlockBuilder::new("cosmos:short_grass", 0.1, 1.0, 0.0).create());

    blocks.register(
        BlockBuilder::new("cosmos:sand", 4.0, 10.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cactus", 0.8, 10.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:build_block", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    // These are the same as COLORS above, but the order matters for stupid reasons. When order no
    // longer matters, please use COLORS instead.
    for &color in [
        "black",
        "grey",
        "white",
        "blue",
        "cyan",
        "brown",
        "green",
        "light_grey",
        "orange",
        "mint",
        "pink",
        "magenta",
        "red",
        "purple",
        "aqua",
        "yellow",
    ]
    .iter()
    {
        blocks.register(
            BlockBuilder::new(format!("cosmos:ship_hull_{color}"), 4.0, 100.0, 10.0)
                .add_property(BlockProperty::Full)
                .create(),
        );
    }

    // Grey registered above to keep id consistency (move down here in future)
    // let ship_hull_colors = [
    //     "black",
    //     "dark_grey",
    //     "white",
    //     "blue",
    //     "dark_blue",
    //     "brown",
    //     "green",
    //     "dark_green",
    //     "orange",
    //     "dark_orange",
    //     "pink",
    //     "dark_pink",
    //     "purple",
    //     "dark_purple",
    //     "red",
    //     "dark_red",
    //     "yellow",
    //     "dark_yellow",
    //     "mint",
    // ];

    // Take 3 because of the skip 3 below. This will eventually not be so stupid, but for now
    // everything will get messed up if I change the order of these being registered.
    for color in COLORS.iter().take(3) {
        blocks.register(
            BlockBuilder::new(format!("cosmos:light_{color}"), 0.1, 20.0, 5.0)
                .add_property(BlockProperty::Full)
                .add_connection_group("cosmos:uses_logic")
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:reactor_controller", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .add_connection_group("cosmos:uses_logic")
            .add_connection_group("cosmos:produces_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_casing", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_window", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:reactor_window")
            .connect_to_group("cosmos:reactor_window")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:reactor_cell", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:fan", 2.0, 20.0, 10.0)
            .add_property(BlockProperty::Transparent)
            .add_property(BlockProperty::Full)
            .create(),
    );

    for color in COLORS.iter() {
        blocks.register(
            BlockBuilder::new(format!("cosmos:glass_{color}"), 4.0, 100.0, 10.0)
                .add_property(BlockProperty::Transparent)
                .add_property(BlockProperty::Full)
                .connect_to_group("cosmos:glass")
                .add_connection_group("cosmos:glass")
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:storage", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:station_core", 2.0, 20.0, 20.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:photonium_crystal_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:plasma_drill", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .add_connection_group("cosmos:uses_logic")
            .add_connection_group("cosmos:consumes_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:shop", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:camera", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:gravity_well", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        // ramp colliders are super small, so to compensate I give them a high density
        BlockBuilder::new("cosmos:ramp_dark_grey", 40.0, 100.0, 10.0)
            .add_property(BlockProperty::FullyRotatable)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:missile_launcher", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::FaceFront)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .add_connection_group("cosmos:consumes_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:shield_projector", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:shield_generator", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:consumes_power")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:logic_on", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:power_cable", 0.1, 20.0, 5.0)
            .add_connection_group("cosmos:power_cable")
            .connect_to_group("cosmos:consumes_power")
            .connect_to_group("cosmos:produces_power")
            .connect_to_group("cosmos:stores_power")
            .connect_to_group("cosmos:power_cable")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:ship_dock", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:tank", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::Transparent)
            .add_connection_group("cosmos:tank")
            .connect_to_group("cosmos:tank")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:logic_indicator", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:and_gate", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:or_gate", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:not_gate", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:xor_gate", 0.1, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FullyRotatable)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    // Buses carry all color signals but cannot go into logic gates (as this would require some implicit reduction to a single signal).
    let mut logic_bus_builder = BlockBuilder::new("cosmos:logic_bus", 0.1, 20.0, 5.0)
        .add_connection_group("cosmos:logic_bus")
        .connect_to_group("cosmos:logic_bus");
    for color in COLORS {
        let colored_wire_name = format!("cosmos:logic_wire_{color}");
        blocks.register(
            BlockBuilder::new(colored_wire_name.clone(), 0.1, 20.0, 5.0)
                .add_connection_group(colored_wire_name.as_ref())
                .connect_to_group(colored_wire_name.as_ref())
                .connect_to_group("cosmos:logic_bus")
                .connect_to_group("cosmos:uses_logic")
                .create(),
        );
        logic_bus_builder = logic_bus_builder.connect_to_group(colored_wire_name.as_ref());
    }

    blocks.register(logic_bus_builder.create());

    blocks.register(
        BlockBuilder::new("cosmos:switch", 5.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:button", 5.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:flip_flop", 5.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .add_connection_group("cosmos:uses_logic")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:door", 4.0, 100.0, 10.0)
            .add_property(BlockProperty::Full)
            .create(),
    );
    blocks.register(
        BlockBuilder::new("cosmos:door_open", 4.0, 100.0, 10.0)
            .add_connection_group("cosmos:door_open")
            .connect_to_group("cosmos:door_open")
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:basic_fabricator", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:iron_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:copper_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:lead_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:uranium_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:sulfur_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:gravitron_crystal_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:energite_crystal_ore", 10.0, 50.0, 12.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    // Skip 3 because of the take 3 above. This will eventually not be so stupid, but for now
    // everything will get messed up if I change the order of these being registered.
    for color in COLORS.iter().skip(3).filter(|x| **x != "white") {
        blocks.register(
            BlockBuilder::new(format!("cosmos:light_{color}"), 0.1, 20.0, 5.0)
                .add_property(BlockProperty::Full)
                .add_connection_group("cosmos:uses_logic")
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:dye_machine", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cloning_bay_base", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cloning_bay_top", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:loot_block", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    for color in COLORS.iter().filter(|x| **x != "dark_grey") {
        blocks.register(
            // ramp colliders are super small, so to compensate I give them a high density
            BlockBuilder::new(format!("cosmos:ramp_{color}"), 40.0, 100.0, 10.0)
                .add_property(BlockProperty::FullyRotatable)
                .create(),
        );
    }

    blocks.register(
        BlockBuilder::new("cosmos:warp_drive", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:warp_disruptor", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:railgun_launcher", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .add_property(BlockProperty::FaceFront)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:magnetic_rail", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:railgun_capacitor", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:cooling_mechanism", 2.0, 20.0, 5.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    blocks.register(
        BlockBuilder::new("cosmos:magnite_ore", 10.0, 50.0, 24.0)
            .add_property(BlockProperty::Full)
            .create(),
    );

    loading.finish_loading(id, &mut end_writer);
}

// Game will break without air & needs this at ID 0
fn add_air_block(
    mut blocks: ResMut<Registry<Block>>,
    mut add_loader_event: EventWriter<AddLoadingEvent>,
    mut done_loading_event: EventWriter<DoneLoadingEvent>,
    mut loader: ResMut<LoadingManager>,
) {
    let id = loader.register_loader(&mut add_loader_event);

    blocks.register(
        BlockBuilder::new("cosmos:air", 0.0, 0.0, 0.0)
            .add_property(BlockProperty::Empty)
            .create(),
    );

    loader.finish_loading(id, &mut done_loading_event);
}

pub(super) fn register<T: States>(app: &mut App, pre_loading_state: T, loading_state: T, post_loading_state: T) {
    registry::create_registry::<Block>(app, "cosmos:blocks");
    fluid::register(app, post_loading_state);

    app.add_systems(OnEnter(pre_loading_state), add_air_block);
    app.add_systems(OnEnter(loading_state), add_cosmos_blocks);
}
