//! Displays any messages the player needs to see

use std::{collections::VecDeque, time::Duration};

use bevy::{
    color::Alpha,
    prelude::{
        in_state, App, AssetServer, BuildChildren, Color, Commands, Component, DespawnRecursiveExt, IntoSystemConfigs, Name, NodeBundle,
        Parent, Query, Res, ResMut, Resource, TextBundle, Update,
    },
    text::{JustifyText, Text, TextSection, TextStyle},
    time::Time,
    ui::{JustifyContent, PositionType, Style, Val},
};

use crate::state::game_state::GameState;

const HUD_DISPLAY_DURATION: Duration = Duration::from_secs(7);
const FADE_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
/// A way of describing colored text used in the HUD message
pub struct RichText {
    /// The color this text should be (default is white)
    pub color: Color,
    /// The text to display
    pub text: String,
}

impl RichText {
    /// Creates a new rich text with these fields
    pub fn new(text: String, color: Color) -> Self {
        Self { color, text }
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            text: "".to_owned(),
        }
    }
}

impl From<String> for RichText {
    fn from(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
/// A message that can be displayed in the HUD
pub struct HudMessage {
    text: Vec<RichText>,
}

impl From<String> for HudMessage {
    fn from(value: String) -> Self {
        Self::with_string(value)
    }
}

impl HudMessage {
    /// The text will be displayed in the order it is given
    ///
    /// Useful for creating multicolored strings
    pub fn new(text: Vec<RichText>) -> Self {
        Self { text }
    }

    /// Creates this from a string - same as `HudMessage::from(String)`
    pub fn with_string(text: String) -> Self {
        Self { text: vec![text.into()] }
    }

    /// Creates this from a colored string
    pub fn with_colored_string(text: String, color: Color) -> Self {
        Self {
            text: vec![RichText { text, color }],
        }
    }
}

#[derive(Debug)]
struct CurrentDisplayedHudCache {
    time_needs_reset: bool,
    cached_message: HudMessage,
}

#[derive(Resource, Debug, Default)]
/// Used to interact with messages shown to the user in the above-hotbar dislay area.
pub struct HudMessages(VecDeque<HudMessage>, Option<CurrentDisplayedHudCache>);

impl HudMessages {
    /// Adds this HUD message to the queue of messages to be displayed, and will display it when it is ready.
    ///
    /// Note that this will check for hud messages with the same text and prevent duplicate entries
    pub fn display_message(&mut self, message: HudMessage) {
        let check_duplicate =
            |m: &HudMessage| m.text.len() == message.text.len() && m.text.iter().zip(message.text.iter()).all(|(x, y)| x.text == y.text);

        if let Some(currently_displayed_hud_cache) = self.1.as_mut() {
            if check_duplicate(&currently_displayed_hud_cache.cached_message) {
                currently_displayed_hud_cache.time_needs_reset = true;
                return;
            }
        }

        if self.0.iter().any(check_duplicate) {
            return;
        }

        self.0.push_back(message);
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct ShownHudMessage {
    time_created: f32,
}

fn display_hud_messages(
    mut commands: Commands,
    mut shown_hud_message: Query<(&Parent, &mut ShownHudMessage, &mut Text)>,
    mut hud_messages: ResMut<HudMessages>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    if let Ok((parent, mut shown_hud_message, mut text)) = shown_hud_message.get_single_mut() {
        let time_now = time.elapsed_seconds();

        if let Some(current_hud_message) = hud_messages.1.as_mut() {
            if current_hud_message.time_needs_reset {
                shown_hud_message.time_created = time_now;
                current_hud_message.time_needs_reset = false;
            }
        }

        let time_remaining = HUD_DISPLAY_DURATION.as_secs_f32() - (time_now - shown_hud_message.time_created);

        if time_remaining <= 0.0 {
            commands.entity(parent.get()).despawn_recursive();
            hud_messages.1 = None;
        } else {
            for section in text.sections.iter_mut() {
                section
                    .style
                    .color
                    .set_alpha((time_remaining / FADE_DURATION.as_secs_f32()).min(1.0));
            }
        }
    } else if let Some(hud_message) = hud_messages.0.pop_front() {
        let shown_hud_message = ShownHudMessage {
            time_created: time.elapsed_seconds(),
        };

        hud_messages.1 = Some(CurrentDisplayedHudCache {
            cached_message: hud_message.clone(),
            time_needs_reset: false,
        });

        commands
            .spawn((
                Name::new("HUD Message"),
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    shown_hud_message,
                    TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(125.0),
                            ..Default::default()
                        },
                        text: Text {
                            sections: hud_message
                                .text
                                .into_iter()
                                .map(|x| TextSection {
                                    value: x.text,
                                    style: TextStyle {
                                        color: x.color,
                                        font: asset_server.load("fonts/PixeloidSans.ttf"),
                                        font_size: 24.0,
                                    },
                                })
                                .collect(),
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));
            });
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<HudMessages>()
        .add_systems(Update, display_hud_messages.run_if(in_state(GameState::Playing)));
}
