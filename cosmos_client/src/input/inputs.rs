use bevy::{prelude::*, utils::HashMap};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum CosmosInputs {
    MoveForward,
    MoveBackward,
    MoveUpOrJump,
    SlowDown,
    MoveLeft,
    MoveRight,
    MoveDown,
    Sprint,

    StopPiloting,
    UseSelectedSystem,

    BreakBlock,
    PlaceBlock,
    Interact,

    CreateShip,

    UnlockMouse,

    SelectSystem1,
    SelectSystem2,
    SelectSystem3,
    SelectSystem4,
    SelectSystem5,
    SelectSystem6,
    SelectSystem7,
    SelectSystem8,
    SelectSystem9,

    HotbarSlot1,
    HotbarSlot2,
    HotbarSlot3,
    HotbarSlot4,
    HotbarSlot5,
    HotbarSlot6,
    HotbarSlot7,
    HotbarSlot8,
    HotbarSlot9,
}

fn init_input(mut input_handler: ResMut<CosmosInputHandler>) {
    // In future load these from settings
    input_handler.set_keycode(CosmosInputs::MoveForward, KeyCode::W);
    input_handler.set_keycode(CosmosInputs::MoveLeft, KeyCode::A);
    input_handler.set_keycode(CosmosInputs::MoveBackward, KeyCode::S);
    input_handler.set_keycode(CosmosInputs::MoveRight, KeyCode::D);
    input_handler.set_keycode(CosmosInputs::SlowDown, KeyCode::LShift);
    input_handler.set_keycode(CosmosInputs::MoveUpOrJump, KeyCode::Space);
    input_handler.set_keycode(CosmosInputs::MoveDown, KeyCode::LShift);
    input_handler.set_keycode(CosmosInputs::Sprint, KeyCode::LControl);

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

    input_handler.set_mouse_button(CosmosInputs::UseSelectedSystem, MouseButton::Left);
}

#[derive(Resource, Default)]
pub struct CosmosInputHandler {
    input_mapping: HashMap<CosmosInputs, (Option<KeyCode>, Option<MouseButton>)>,
}

impl CosmosInputHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_just_released(
        &self,
        input_code: CosmosInputs,
        inputs: &Input<KeyCode>,
        mouse: &Input<MouseButton>,
    ) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_released(keycode.unwrap())
            || mouse_button.is_some() && mouse.just_released(mouse_button.unwrap())
    }

    pub fn check_released(
        &self,
        input_code: CosmosInputs,
        inputs: &Input<KeyCode>,
        mouse: &Input<MouseButton>,
    ) -> bool {
        !self.check_pressed(input_code, inputs, mouse)
    }

    pub fn check_just_pressed(
        &self,
        input_code: CosmosInputs,
        inputs: &Input<KeyCode>,
        mouse: &Input<MouseButton>,
    ) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && inputs.just_pressed(keycode.unwrap())
            || mouse_button.is_some() && mouse.just_pressed(mouse_button.unwrap())
    }

    pub fn check_pressed(
        &self,
        input_code: CosmosInputs,
        keys: &Input<KeyCode>,
        mouse: &Input<MouseButton>,
    ) -> bool {
        let keycode = self.keycode_for(input_code);
        let mouse_button = self.mouse_button_for(input_code);

        keycode.is_some() && keys.pressed(keycode.unwrap())
            || mouse_button.is_some() && mouse.pressed(mouse_button.unwrap())
    }

    pub fn clear_input(&mut self, input: CosmosInputs) {
        self.input_mapping.remove(&input);
    }

    pub fn set_keycode(&mut self, input: CosmosInputs, keycode: KeyCode) {
        if self.input_mapping.contains_key(&input) {
            let mapping = self.input_mapping.get_mut(&input).unwrap();

            mapping.0 = Some(keycode);
            mapping.1 = None;
        } else {
            self.input_mapping.insert(input, (Some(keycode), None));
        }
    }

    pub fn set_mouse_button(&mut self, input: CosmosInputs, button: MouseButton) {
        if self.input_mapping.contains_key(&input) {
            let mapping = self.input_mapping.get_mut(&input).unwrap();

            mapping.0 = None;
            mapping.1 = Some(button);
        } else {
            self.input_mapping.insert(input, (None, Some(button)));
        }
    }

    pub fn keycode_for(&self, input: CosmosInputs) -> Option<KeyCode> {
        if !self.input_mapping.contains_key(&input) {
            return None;
        }

        self.input_mapping[&input].0
    }

    pub fn mouse_button_for(&self, input: CosmosInputs) -> Option<MouseButton> {
        if !self.input_mapping.contains_key(&input) {
            return None;
        }

        self.input_mapping[&input].1
    }
}

pub fn register(app: &mut App) {
    app.insert_resource(CosmosInputHandler::new())
        .add_startup_system(init_input);
}
