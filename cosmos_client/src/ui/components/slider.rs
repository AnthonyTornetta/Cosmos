use std::ops::Range;

use bevy::{
    app::{App, Update},
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        query::{Added, Changed},
        schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query},
    },
    hierarchy::Children,
    render::color::Color,
    text::Text,
    ui::{node_bundles::NodeBundle, BackgroundColor, Interaction},
};

#[derive(Component, Debug)]
pub struct Slider {
    pub slider_styles: Option<SliderStyles>,
    pub range: Range<i64>,
}

#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SliderValue(i64);

impl SliderValue {
    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn set_value(&mut self, new_val: i64) {
        self.0 = new_val;
    }
}

#[derive(Default, Debug)]
pub struct SliderStyles {
    pub background_color: Color,
    pub foreground_color: Color,

    pub hover_background_color: Color,
    pub hover_foreground_color: Color,

    pub press_background_color: Color,
    pub press_foreground_color: Color,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            range: 0..101,
            slider_styles: Default::default(),
        }
    }
}

#[derive(Debug, Bundle, Default)]
pub struct SliderBundle {
    /// The node bundle that will be used with the TextInput
    pub node_bundle: NodeBundle,
    pub slider: Slider,
    pub slider_value: SliderValue,
}

fn on_add_slider(mut commands: Commands, mut q_added_button: Query<(Entity, &mut Slider, &SliderValue), Added<Slider>>) {
    for (ent, mut slider, slider_value) in q_added_button.iter_mut() {
        commands.entity(ent).insert(Interaction::default());
    }
}

fn on_interact_slider(
    mut q_added_button: Query<(&Interaction, &Slider, &mut SliderValue, &mut BackgroundColor, &Children), Changed<Interaction>>,
    mut q_text: Query<&mut Text>,
) {
    for (interaction, slider, slider_value, mut bg_color, children) in q_added_button.iter_mut() {
        if let Some(slider_styles) = &slider.slider_styles {
            bg_color.0 = match *interaction {
                Interaction::None => slider_styles.background_color,
                Interaction::Hovered => slider_styles.hover_background_color,
                Interaction::Pressed => slider_styles.press_background_color,
            };

            if let Some(&text_child) = children.iter().find(|&x| q_text.contains(*x)) {
                let mut text = q_text.get_mut(text_child).expect("Checked above");

                let color = match *interaction {
                    Interaction::None => slider_styles.foreground_color,
                    Interaction::Hovered => slider_styles.hover_foreground_color,
                    Interaction::Pressed => slider_styles.press_foreground_color,
                };

                text.sections.iter_mut().for_each(|x| x.style.color = color);
            }
        }
    }
}

// https://github.com/bevyengine/bevy/pull/9822
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`Button`]` component uses. Make sure you add any [`Button`] components before this set!
pub enum SliderUiSystemSet {
    /// apply_deferred
    ApplyDeferredA,
    /// Make sure you add any [`Button`] components before this set!
    ///
    /// Sets up any [`Button`] components added.
    AddSliderBundle,
    /// apply_deferred
    ApplyDeferredB,
    /// Sends user events from the various [`Button`] components.
    UpdateSliderValues,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            SliderUiSystemSet::ApplyDeferredA,
            SliderUiSystemSet::AddSliderBundle,
            SliderUiSystemSet::ApplyDeferredB,
            SliderUiSystemSet::UpdateSliderValues,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(SliderUiSystemSet::ApplyDeferredA),
            apply_deferred.in_set(SliderUiSystemSet::ApplyDeferredB),
        ),
    )
    .add_systems(
        Update,
        (
            on_add_slider.in_set(SliderUiSystemSet::AddSliderBundle),
            on_interact_slider.in_set(SliderUiSystemSet::UpdateSliderValues),
        ),
    );
}
