//! A checkbox UI element
//!
//! Use [`checkbox`] to create one.

use bevy::{color::palettes::css, prelude::*};

use crate::ui::UiSystemSet;

#[derive(Component, Default, Debug)]
#[require(Node)]
/// A checkable UI element
pub enum Checkbox {
    /// The checkbox is checked
    Enabled,
    #[default]
    /// The checkbox is not checked
    Disabled,
}

impl Checkbox {
    /// Checks whether this is checked as a true/false value
    pub fn get(&self) -> bool {
        match *self {
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }

    /// Toggles this checkbox's state
    pub fn toggle(&mut self) {
        *self = match *self {
            Self::Enabled => Self::Disabled,
            Self::Disabled => Self::Enabled,
        };
    }
}

/// Creates a checkbox with this initial state
pub fn checkbox(checkbox: Checkbox) -> impl Bundle {
    return (
        Node {
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            border: UiRect::all(Val::Px(2.0)),
            ..Default::default()
        },
        BackgroundColor(css::DARK_GRAY.into()),
        BorderColor::all(css::LIGHT_GREY),
        checkbox,
        Pickable::default(),
    );
}

fn on_add_checkbox(q_added: Query<Entity, Added<Checkbox>>, mut commands: Commands) {
    for e in q_added.iter() {
        commands
            .entity(e)
            .observe(|on: On<Pointer<Click>>, mut q_checkbox: Query<&mut Checkbox>| {
                let Ok(mut cb) = q_checkbox.get_mut(on.entity) else {
                    return;
                };

                cb.toggle();
            });
    }
}

fn on_change_checkbox(mut q_changed: Query<(&Checkbox, &mut BackgroundColor), Changed<Checkbox>>) {
    for (cb, mut bg) in q_changed.iter_mut() {
        match cb {
            Checkbox::Enabled => {
                bg.0 = css::AQUA.into();
            }
            Checkbox::Disabled => {
                bg.0 = css::DARK_GREY.into();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (on_add_checkbox, on_change_checkbox).chain().in_set(UiSystemSet::DoUi));
}
