//! Logic behavior for "Numeric Display", a block that displays a digit 0-9 representing the logic signal it's recieving.

use bevy::prelude::*;

use cosmos_core::{
    block::{
        Block, block_face::BlockFace, block_rotation::BlockRotation, data::BlockData, specific_blocks::numeric_display::NumericDisplayValue,
    },
    events::block_events::BlockChangedMessage,
    prelude::BlockCoordinate,
    registry::{Registry, identifiable::Identifiable},
    structure::{Structure, coordinates::BoundsError},
};

use crate::logic::{BlockLogicData, LogicBlock, LogicConnection, LogicInputMessage, LogicSystemSet, PortType, logic_driver::LogicDriver};

fn register_logic_ports(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<LogicBlock>>) {
    if let Some(numeric_display) = blocks.from_id("cosmos:numeric_display") {
        registry.register(LogicBlock::new(
            numeric_display,
            [None, Some(LogicConnection::Port(PortType::Input)), None, None, None, None],
        ));
    }
}

/// Handles changes to the input value of "root" displays (those show the "ones" digit).
///
/// Also handles updating child display values after a display is placed next to an
/// existing root, since its logic input of 0 is processed after it's placed.
fn numeric_display_input_event_listener(
    mut evr_logic_input: MessageReader<LogicInputMessage>,
    blocks: Res<Registry<Block>>,
    mut q_structure_logic_driver: Query<(&mut Structure, &mut LogicDriver)>,
    mut q_logic_data: Query<&mut BlockLogicData>,
    mut q_numeric_display_value: Query<&mut NumericDisplayValue>,
    mut commands: Commands,
    q_has_logic_data: Query<(), With<BlockLogicData>>,
    q_has_display_value: Query<(), With<NumericDisplayValue>>,
    mut q_block_data: Query<&mut BlockData>,
) {
    for ev in evr_logic_input.read() {
        let Ok((mut structure, logic_driver)) = q_structure_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let coords = ev.block.coords();
        if structure.block_at(coords, &blocks).unlocalized_name() != "cosmos:numeric_display" {
            continue;
        }

        // Sets the block's logic data, not necessary for rendering.
        let rotation = structure.block_rotation(coords);
        let logic_value = BlockLogicData(logic_driver.read_input(coords, rotation.direction_of(BlockFace::Left)));
        if let Some(mut logic_data) = structure.query_block_data_mut(coords, &mut q_logic_data, &mut commands) {
            if **logic_data != logic_value {
                **logic_data = logic_value;
            }
        } else if logic_value.0 != 0 {
            structure.insert_block_data(coords, logic_value, &mut commands, &mut q_block_data, &q_has_logic_data);
        }

        // The root numeric display is the leftmost one in the line (with the exposed input port).
        let rotation = structure.block_rotation(coords);
        let left_direction = rotation.direction_of(BlockFace::Left);
        let mut steps_to_root = 0;
        let mut root_coords = coords;
        let mut check_coords = coords.step(left_direction);
        while let Some(display_coords) = check_for_aligned_display(&check_coords, rotation, &structure, &blocks) {
            steps_to_root += 1;
            root_coords = display_coords;
            check_coords = root_coords.step(left_direction);
        }

        update_child_displays(
            coords,
            root_coords,
            steps_to_root,
            &mut structure,
            &logic_driver,
            &mut q_numeric_display_value,
            &mut q_block_data,
            &q_has_display_value,
            &mut commands,
            &blocks,
        );
    }
}

/// Handles updating child display values after a display closer to the root is broken.
fn numeric_display_block_broken_event_listener(
    mut evr_block_changed: MessageReader<BlockChangedMessage>,
    blocks: Res<Registry<Block>>,
    mut q_structure_logic_driver: Query<(&mut Structure, &mut LogicDriver)>,
    mut q_numeric_display_value: Query<&mut NumericDisplayValue>,
    q_has_display_value: Query<(), With<NumericDisplayValue>>,
    mut q_block_data: Query<&mut BlockData>,
    mut commands: Commands,
) {
    for ev in evr_block_changed.read() {
        if blocks.from_numeric_id(ev.old_block).unlocalized_name() != "cosmos:numeric_display" {
            continue;
        };
        let Ok((mut structure, logic_driver)) = q_structure_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };

        let coords = ev.block.coords();
        let rotation = ev.old_block_rotation();
        let check_coords = coords.step(rotation.direction_of(BlockFace::Right));

        let Some(root_coords) = check_for_aligned_display(&check_coords, rotation, &structure, &blocks) else {
            continue;
        };

        update_child_displays(
            root_coords,
            root_coords,
            0,
            &mut structure,
            &logic_driver,
            &mut q_numeric_display_value,
            &mut q_block_data,
            &q_has_display_value,
            &mut commands,
            &blocks,
        );
    }
}

fn update_child_displays(
    coords: BlockCoordinate,
    root_coords: BlockCoordinate,
    steps_to_root: usize,
    structure: &mut Structure,
    logic_driver: &LogicDriver,
    q_numeric_display_value: &mut Query<&mut NumericDisplayValue>,
    q_block_data: &mut Query<&mut BlockData>,
    q_has_display_value: &Query<(), With<NumericDisplayValue>>,
    commands: &mut Commands,
    blocks: &Registry<Block>,
) {
    let rotation = structure.block_rotation(coords);
    let root_display_logic_value = BlockLogicData(logic_driver.read_input(root_coords, rotation.direction_of(BlockFace::Left)));

    // Updates the display value on the numeric display from the current event.
    let display_string = root_display_logic_value.0.to_string();
    let mut character_iterator = display_string.chars().rev();
    display_character_at(
        character_iterator.nth(steps_to_root),
        coords,
        structure,
        q_numeric_display_value,
        q_block_data,
        q_has_display_value,
        commands,
    );

    // Updates the display values of every numeric display to the right.
    let right_direction = rotation.direction_of(BlockFace::Right);
    let mut check_coords = coords.step(right_direction);
    while let Some(display_coords) = check_for_aligned_display(&check_coords, rotation, structure, blocks) {
        display_character_at(
            character_iterator.next(),
            display_coords,
            structure,
            q_numeric_display_value,
            q_block_data,
            q_has_display_value,
            commands,
        );
        check_coords = display_coords.step(right_direction);
    }
}

fn check_for_aligned_display(
    check_coords: &Result<BlockCoordinate, BoundsError>,
    rotation: BlockRotation,
    structure: &Structure,
    blocks: &Registry<Block>,
) -> Option<BlockCoordinate> {
    let Ok(coords) = *check_coords else {
        return None;
    };
    if structure.block_at(coords, blocks).unlocalized_name() == "cosmos:numeric_display" && structure.block_rotation(coords) == rotation {
        return Some(coords);
    }
    None
}

fn display_character_at(
    character: Option<char>,
    coords: BlockCoordinate,
    structure: &mut Structure,
    q_numeric_display_value: &mut Query<&mut NumericDisplayValue>,
    q_block_data: &mut Query<&mut BlockData>,
    q_has_display_value: &Query<(), With<NumericDisplayValue>>,
    commands: &mut Commands,
) {
    let display_value = match character {
        None => NumericDisplayValue::Blank,
        Some('0') => NumericDisplayValue::Zero,
        Some('1') => NumericDisplayValue::One,
        Some('2') => NumericDisplayValue::Two,
        Some('3') => NumericDisplayValue::Three,
        Some('4') => NumericDisplayValue::Four,
        Some('5') => NumericDisplayValue::Five,
        Some('6') => NumericDisplayValue::Six,
        Some('7') => NumericDisplayValue::Seven,
        Some('8') => NumericDisplayValue::Eight,
        Some('9') => NumericDisplayValue::Nine,
        Some('-') => NumericDisplayValue::Minus,
        _ => unreachable!("Logic signal should not contain characters other than '-' and 0-9"),
    };
    if let Some(mut numeric_display_data) = structure.query_block_data_mut(coords, q_numeric_display_value, commands) {
        if **numeric_display_data != display_value {
            **numeric_display_data = display_value;
        }
    } else {
        structure.insert_block_data(coords, display_value, commands, q_block_data, q_has_display_value);
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(OnEnter(post_loading_state), register_logic_ports)
        .add_systems(
            FixedUpdate,
            numeric_display_input_event_listener
                .in_set(LogicSystemSet::Consume)
                .ambiguous_with(LogicSystemSet::Consume),
        )
        .add_systems(
            FixedUpdate,
            numeric_display_block_broken_event_listener.in_set(LogicSystemSet::EditLogicGraph),
        );
}
