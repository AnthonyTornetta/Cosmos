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

    // These two controls will eventually be removed
    /// Create a ship with a ship core in the player's inventory
    CreateShip,
    /// Creates a space station with the station core in the player's inventory
    CreateStation,

    /// Unlocks the mouse from the window
    UnlockMouse,

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

    /// Toggles the player between being in build mode and not on a ship
    ToggleBuildMode,
    /// A toggle to clear the symmetry - when combined with a symmetry key the symmetry will be cleared
    ClearSymmetry,
    /// Creates an X symmetry
    SymmetryX,
    /// Creates a Y symmetry
    SymmetryY,
    /// Creates a Z symmetry
    SymmetryZ,

    /// Focuses/unfofcuses the waypoint the player is looking at
    FocusWaypoint,

    /// Changes which camera is selected in a ship
    SwapCameraLeft,
    /// Changes which camera is selected in a ship
    SwapCameraRight,
}

fn init_input(mut input_handler: ResMut<CosmosInputHandler>) {
    // In future load these from settings
    input_handler.set_keycode(CosmosInputs::MoveForward, KeyCode::KeyW);
    input_handler.set_keycode(CosmosInputs::MoveLeft, KeyCode::KeyA);
    input_handler.set_keycode(CosmosInputs::MoveBackward, KeyCode::KeyS);
    input_handler.set_keycode(CosmosInputs::MoveRight, KeyCode::KeyD);
    input_handler.set_keycode(CosmosInputs::SlowDown, KeyCode::ShiftLeft);
    input_handler.set_keycode(CosmosInputs::Jump, KeyCode::Space);
    input_handler.set_keycode(CosmosInputs::MoveDown, KeyCode::KeyQ);
    input_handler.set_keycode(CosmosInputs::MoveUp, KeyCode::KeyE);
    input_handler.set_keycode(CosmosInputs::Sprint, KeyCode::ControlLeft);

    input_handler.set_keycode(CosmosInputs::RollLeft, KeyCode::KeyZ);
    input_handler.set_keycode(CosmosInputs::RollRight, KeyCode::KeyC);

    input_handler.set_mouse_button(CosmosInputs::BreakBlock, MouseButton::Left);
    input_handler.set_mouse_button(CosmosInputs::PlaceBlock, MouseButton::Right);
    input_handler.set_keycode(CosmosInputs::Interact, KeyCode::KeyR);
    input_handler.set_keycode(CosmosInputs::StopPiloting, KeyCode::KeyR);

    input_handler.set_keycode(CosmosInputs::CreateShip, KeyCode::KeyX);
    input_handler.set_keycode(CosmosInputs::CreateStation, KeyCode::KeyY);

    input_handler.set_keycode(CosmosInputs::UnlockMouse, KeyCode::Escape);

    input_handler.set_keycode(CosmosInputs::HotbarSlot1, KeyCode::Digit1);
    input_handler.set_keycode(CosmosInputs::HotbarSlot2, KeyCode::Digit2);
    input_handler.set_keycode(CosmosInputs::HotbarSlot3, KeyCode::Digit3);
    input_handler.set_keycode(CosmosInputs::HotbarSlot4, KeyCode::Digit4);
    input_handler.set_keycode(CosmosInputs::HotbarSlot5, KeyCode::Digit5);
    input_handler.set_keycode(CosmosInputs::HotbarSlot6, KeyCode::Digit6);
    input_handler.set_keycode(CosmosInputs::HotbarSlot7, KeyCode::Digit7);
    input_handler.set_keycode(CosmosInputs::HotbarSlot8, KeyCode::Digit8);
    input_handler.set_keycode(CosmosInputs::HotbarSlot9, KeyCode::Digit9);

    input_handler.set_keycode(CosmosInputs::Disconnect, KeyCode::KeyP);

    input_handler.set_mouse_button(CosmosInputs::UseSelectedSystem, MouseButton::Left);

    input_handler.set_keycode(CosmosInputs::LeaveShip, KeyCode::KeyL);

    input_handler.set_keycode(CosmosInputs::ToggleInventory, KeyCode::KeyT);
    input_handler.set_keycode(CosmosInputs::AutoMoveItem, KeyCode::ShiftLeft);

    input_handler.set_keycode(CosmosInputs::ToggleBuildMode, KeyCode::KeyB);
    input_handler.set_keycode(CosmosInputs::ClearSymmetry, KeyCode::ShiftLeft);
    input_handler.set_keycode(CosmosInputs::SymmetryX, KeyCode::KeyX);
    input_handler.set_keycode(CosmosInputs::SymmetryY, KeyCode::KeyY);
    input_handler.set_keycode(CosmosInputs::SymmetryZ, KeyCode::KeyZ);

    input_handler.set_keycode(CosmosInputs::FocusWaypoint, KeyCode::KeyF);

    input_handler.set_keycode(CosmosInputs::SwapCameraLeft, KeyCode::ArrowLeft);
    input_handler.set_keycode(CosmosInputs::SwapCameraRight, KeyCode::ArrowRight);
}

#[derive(Resource, Default, Debug)]
/// Use this to check if inputs are selected
///
/// You should generally prefer to use the `InputChecker` unless you're doing something super specific.
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

    /// Gets the raw mouse key structure (Res<ButtonInput<KeyCode>>)
    fn key_inputs(&self) -> &ButtonInput<KeyCode>;

    /// Gets the raw mouse inputs structure (Res<ButtonInput<KeyCode>>)
    fn mouse_inputs(&self) -> &ButtonInput<MouseButton>;
}

/// A wrapper around [`CosmosInputHandler`] and all the resources it needs.
///
/// It just makes calling the functions a little bit easier
pub type InputChecker<'a> = (
    Res<'a, CosmosInputHandler>,
    Res<'a, ButtonInput<KeyCode>>,
    Res<'a, ButtonInput<MouseButton>>,
);

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

    fn key_inputs(&self) -> &ButtonInput<KeyCode> {
        &self.1
    }

    fn mouse_inputs(&self) -> &ButtonInput<MouseButton> {
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
    pub fn check_just_released(&self, input_code: CosmosInputs, inputs: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_released(keycode.unwrap()) || mouse_button.is_some() && mouse.just_released(mouse_button.unwrap())
    }

    /// Check if the given input is not being used.
    pub fn check_released(&self, input_code: CosmosInputs, inputs: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        !self.check_pressed(input_code, inputs, mouse)
    }

    /// Checks if the given input was just pressed.
    ///
    /// Use this to see if something was pressed just this frame.
    pub fn check_just_pressed(&self, input_code: CosmosInputs, inputs: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_pressed(keycode.unwrap()) || mouse_button.is_some() && mouse.just_pressed(mouse_button.unwrap())
    }

    /// Check if this input is currently being used.
    pub fn check_pressed(&self, input_code: CosmosInputs, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
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
