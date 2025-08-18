use bevy::prelude::*;

use crate::ui::{OpenMenu, components::window::GuiWindow};

pub mod confirm_modal;
pub mod text_modal;

#[derive(Component, Default)]
#[require(Node)]
pub struct Modal {
    title: String,
}

#[derive(Component)]
pub struct ModalBody(Entity);

pub const MODAL_MENU_LEVEL: u32 = 10;

fn on_add_modal(mut commands: Commands, q_modal: Query<(&mut Node, Entity, &Modal), Added<Modal>>) {
    for (node, ent, modal) in q_modal.iter() {
        let body_node = node.clone();
        let modal_body = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                Name::new("Modal Body"),
            ))
            .id();

        commands
            .entity(ent)
            .insert((
                GuiWindow {
                    title: modal.title.clone(),
                    body_styles: body_node,
                    ..Default::default()
                },
                Node {
                    margin: UiRect::all(Val::Auto),
                    position_type: PositionType::Absolute,
                    width: Val::Px(600.0),
                    height: Val::Px(200.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                OpenMenu::new(MODAL_MENU_LEVEL),
                ModalBody(modal_body),
            ))
            .add_child(modal_body);
    }
}

pub(super) fn register(app: &mut App) {
    text_modal::register(app);
    confirm_modal::register(app);

    app.add_systems(Update, on_add_modal);
}
