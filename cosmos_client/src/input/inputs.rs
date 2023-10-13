//! Represents the cosmos input systems

use bevy::{prelude::*, utils::HashMap};

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
/// This should be refactored into a registry, but for now, enjoy enum!
///
/// Use this for input handling to allow things to be automatically changed
pub enum CosmosInputs {
    /// Player/Ship move forward
    MoveForward,
    /// Player/Ship move backward
    MoveBackward,
    /// Player jump
    Jump,
    /// Player/Ship slow down
    SlowDown,
    /// Player/Ship move left
    MoveLeft,
    /// Player/Ship move right
    MoveRight,
    /// Player move faster
    Sprint,

    // For use in ships
    /// Ship move down
    MoveDown,
    /// Ship move up
    MoveUp,
    /// Ship roll left
    RollLeft,
    /// Ship roll right
    RollRight,
    /// Leaves the ship the player is a child of
    ///
    /// This does not remove you as the pilot, but rather makes you no longer
    /// move with the ship
    LeaveShip,

    /// Stop piloting whatever ship they're in
    StopPiloting,
    /// Use the ship's selected block system
    UseSelectedSystem,

    /// Break the block the player is looking at
    BreakBlock,
    /// Place the block the player is holding
    PlaceBlock,
    /// Interact with the block the player is looking at
    Interact,

    /// Create a ship with a ship core in the player's inventory
    CreateShip,

    /// Unlocks the mouse from the window
    UnlockMouse,

    /// Change the selected block system while piloting ship
    SelectSystem1,
    /// Change the selected block system while piloting ship
    SelectSystem2,
    /// Change the selected block system while piloting ship
    SelectSystem3,
    /// Change the selected block system while piloting ship
    SelectSystem4,
    /// Change the selected block system while piloting ship
    SelectSystem5,
    /// Change the selected block system while piloting ship
    SelectSystem6,
    /// Change the selected block system while piloting ship
    SelectSystem7,
    /// Change the selected block system while piloting ship
    SelectSystem8,
    /// Change the selected block system while piloting ship
    SelectSystem9,

    /// Change the selected inventory item
    HotbarSlot1,
    /// Change the selected inventory item
    HotbarSlot2,
    /// Change the selected inventory item
    HotbarSlot3,
    /// Change the selected inventory item
    HotbarSlot4,
    /// Change the selected inventory item
    HotbarSlot5,
    /// Change the selected inventory item
    HotbarSlot6,
    /// Change the selected inventory item
    HotbarSlot7,
    /// Change the selected inventory item
    HotbarSlot8,
    /// Change the selected inventory item
    HotbarSlot9,

    /// For testing - disconnects you from the server
    Disconnect,

    /// Opens + closes your inventory
    ToggleInventory,
    /// "Shift-Clicking" an item in minecraft
    AutoMoveItem,
}

fn init_input(mut input_handler: ResMut<CosmosInputHandler>) {
    // In future load these from settings
    input_handler.set_keycode(CosmosInputs::MoveForward, KeyCode::W);
    input_handler.set_keycode(CosmosInputs::MoveLeft, KeyCode::A);
    input_handler.set_keycode(CosmosInputs::MoveBackward, KeyCode::S);
    input_handler.set_keycode(CosmosInputs::MoveRight, KeyCode::D);
    input_handler.set_keycode(CosmosInputs::SlowDown, KeyCode::ShiftLeft);
    input_handler.set_keycode(CosmosInputs::Jump, KeyCode::Space);
    input_handler.set_keycode(CosmosInputs::MoveDown, KeyCode::Q);
    input_handler.set_keycode(CosmosInputs::MoveUp, KeyCode::E);
    input_handler.set_keycode(CosmosInputs::Sprint, KeyCode::ControlLeft);

    input_handler.set_keycode(CosmosInputs::RollLeft, KeyCode::Z);
    input_handler.set_keycode(CosmosInputs::RollRight, KeyCode::C);

    input_handler.set_mouse_button(CosmosInputs::BreakBlock, MouseButton::Left);
    input_handler.set_mouse_button(CosmosInputs::PlaceBlock, MouseButton::Right);
    input_handler.set_keycode(CosmosInputs::Interact, KeyCode::R);
    input_handler.set_keycode(CosmosInputs::StopPiloting, KeyCode::R);

    input_handler.set_keycode(CosmosInputs::CreateShip, KeyCode::X);

    input_handler.set_keycode(CosmosInputs::UnlockMouse, KeyCode::Escape);

    input_handler.set_keycode(CosmosInputs::HotbarSlot1, KeyCode::Key1);
    input_handler.set_keycode(CosmosInputs::HotbarSlot2, KeyCode::Key2);
    input_handler.set_keycode(CosmosInputs::HotbarSlot3, KeyCode::Key3);
    input_handler.set_keycode(CosmosInputs::HotbarSlot4, KeyCode::Key4);
    input_handler.set_keycode(CosmosInputs::HotbarSlot5, KeyCode::Key5);
    input_handler.set_keycode(CosmosInputs::HotbarSlot6, KeyCode::Key6);
    input_handler.set_keycode(CosmosInputs::HotbarSlot7, KeyCode::Key7);
    input_handler.set_keycode(CosmosInputs::HotbarSlot8, KeyCode::Key8);
    input_handler.set_keycode(CosmosInputs::HotbarSlot9, KeyCode::Key9);

    input_handler.set_keycode(CosmosInputs::SelectSystem1, KeyCode::Key1);
    input_handler.set_keycode(CosmosInputs::SelectSystem2, KeyCode::Key2);
    input_handler.set_keycode(CosmosInputs::SelectSystem3, KeyCode::Key3);
    input_handler.set_keycode(CosmosInputs::SelectSystem4, KeyCode::Key4);
    input_handler.set_keycode(CosmosInputs::SelectSystem5, KeyCode::Key5);
    input_handler.set_keycode(CosmosInputs::SelectSystem6, KeyCode::Key6);
    input_handler.set_keycode(CosmosInputs::SelectSystem7, KeyCode::Key7);
    input_handler.set_keycode(CosmosInputs::SelectSystem8, KeyCode::Key8);
    input_handler.set_keycode(CosmosInputs::SelectSystem9, KeyCode::Key9);

    input_handler.set_keycode(CosmosInputs::Disconnect, KeyCode::P);

    input_handler.set_mouse_button(CosmosInputs::UseSelectedSystem, MouseButton::Left);

    input_handler.set_keycode(CosmosInputs::LeaveShip, KeyCode::L);

    input_handler.set_keycode(CosmosInputs::ToggleInventory, KeyCode::T);
    input_handler.set_keycode(CosmosInputs::AutoMoveItem, KeyCode::ShiftLeft);
}

#[derive(Resource, Default, Debug)]
/// Use this to check if inputs are selected
pub struct CosmosInputHandler {
    input_mapping: HashMap<CosmosInputs, (Option<KeyCode>, Option<MouseButton>)>,
}

/// A wrapper around [`CosmosInputHandler`] and all the resources it needs.
///
/// It just makes calling the functions a little bit easier
pub trait InputHandler {
    /// Check if the given input was just released.
    ///
    /// Use this to see if something was held in the last frame but is no longer being held.
    fn check_just_released(&self, input_code: CosmosInputs) -> bool;

    /// Check if the given input is not being used.
    fn check_released(&self, input_code: CosmosInputs) -> bool;

    /// Checks if the given input was just pressed.
    ///
    /// Use this to see if something was pressed just this frame.
    fn check_just_pressed(&self, input_code: CosmosInputs) -> bool;

    /// Check if this input is currently being used.
    fn check_pressed(&self, input_code: CosmosInputs) -> bool;

    /// Gets the raw mouse key structure (Res<Input<KeyCode>>)
    fn key_inputs(&self) -> &Input<KeyCode>;

    /// Gets the raw mouse inputs structure (Res<Input<KeyCode>>)
    fn mouse_inputs(&self) -> &Input<MouseButton>;
}

/// A wrapper around [`CosmosInputHandler`] and all the resources it needs.
///
/// It just makes calling the functions a little bit easier
pub type InputChecker<'a> = (Res<'a, CosmosInputHandler>, Res<'a, Input<KeyCode>>, Res<'a, Input<MouseButton>>);

impl<'a> InputHandler for InputChecker<'a> {
    fn check_just_pressed(&self, input_code: CosmosInputs) -> bool {
        self.0.check_just_pressed(input_code, &self.1, &self.2)
    }

    fn check_just_released(&self, input_code: CosmosInputs) -> bool {
        self.0.check_just_released(input_code, &self.1, &self.2)
    }

    fn check_pressed(&self, input_code: CosmosInputs) -> bool {
        self.0.check_pressed(input_code, &self.1, &self.2)
    }

    fn check_released(&self, input_code: CosmosInputs) -> bool {
        self.0.check_released(input_code, &self.1, &self.2)
    }

    fn key_inputs(&self) -> &Input<KeyCode> {
        &self.1
    }

    fn mouse_inputs(&self) -> &Input<MouseButton> {
        &self.2
    }
}

impl CosmosInputHandler {
    /// Default
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the given input was just released.
    ///
    /// Use this to see if something was held in the last frame but is no longer being held.
    pub fn check_just_released(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>, mouse: &Input<MouseButton>) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_released(keycode.unwrap()) || mouse_button.is_some() && mouse.just_released(mouse_button.unwrap())
    }

    /// Check if the given input is not being used.
    pub fn check_released(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>, mouse: &Input<MouseButton>) -> bool {
        !self.check_pressed(input_code, inputs, mouse)
    }

    /// Checks if the given input was just pressed.
    ///
    /// Use this to see if something was pressed just this frame.
    pub fn check_just_pressed(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>, mouse: &Input<MouseButton>) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_pressed(keycode.unwrap()) || mouse_button.is_some() && mouse.just_pressed(mouse_button.unwrap())
    }

    /// Check if this input is currently being used.
    pub fn check_pressed(&self, input_code: CosmosInputs, keys: &Input<KeyCode>, mouse: &Input<MouseButton>) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && keys.pressed(keycode.unwrap()) || mouse_button.is_some() && mouse.pressed(mouse_button.unwrap())
    }

    fn set_keycode(&mut self, input: CosmosInputs, keycode: KeyCode) {
        if self.input_mapping.contains_key(&input) {
            let mapping = self.input_mapping.get_mut(&input).unwrap();

            mapping.0 = Some(keycode);
            mapping.1 = None;
        } else {
            self.input_mapping.insert(input, (Some(keycode), None));
        }
    }

    fn set_mouse_button(&mut self, input: CosmosInputs, button: MouseButton) {
        if self.input_mapping.contains_key(&input) {
            let mapping = self.input_mapping.get_mut(&input).unwrap();

            mapping.0 = None;
            mapping.1 = Some(button);
        } else {
            self.input_mapping.insert(input, (None, Some(button)));
        }
    }

    fn keycode_for(&self, input: CosmosInputs) -> Option<KeyCode> {
        if !self.input_mapping.contains_key(&input) {
            return None;
        }

        self.input_mapping[&input].0
    }

    fn mouse_button_for(&self, input: CosmosInputs) -> Option<MouseButton> {
        if !self.input_mapping.contains_key(&input) {
            return None;
        }

        self.input_mapping[&input].1
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CosmosInputHandler::new()).add_systems(Startup, init_input);
}
