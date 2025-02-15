//! Displays any messages the player needs to see

use std::{collections::VecDeque, time::Duration};

use bevy::prelude::*;
use cosmos_core::{netty::system_sets::NetworkingSystemsSet, state::GameState};

use super::font::DefaultFont;

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
    pub fn with_string(text: impl Into<String>) -> Self {
        Self {
            text: vec![text.into().into()],
        }
    }

    /// Creates this from a colored string
    pub fn with_colored_string(text: impl Into<String>, color: Color) -> Self {
        Self {
            text: vec![RichText { text: text.into(), color }],
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
    default_font: Res<DefaultFont>,
    mut commands: Commands,
    mut shown_hud_message: Query<(Entity, &Parent, &mut ShownHudMessage)>,
    mut hud_messages: ResMut<HudMessages>,
    mut writer: TextUiWriter,
    time: Res<Time>,
) {
    if let Ok((entity, parent, mut shown_hud_message)) = shown_hud_message.get_single_mut() {
        let time_now = time.elapsed_secs();

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
            writer.for_each_color(entity, |mut c| c.set_alpha((time_remaining / FADE_DURATION.as_secs_f32()).min(1.0)));
        }
    } else if let Some(hud_message) = hud_messages.0.pop_front() {
        let shown_hud_message = ShownHudMessage {
            time_created: time.elapsed_secs(),
        };

        hud_messages.1 = Some(CurrentDisplayedHudCache {
            cached_message: hud_message.clone(),
            time_needs_reset: false,
        });

        commands
            .spawn((
                Name::new("HUD Message"),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                // TODO: This
                let mut messages = hud_message.text.into_iter();
                if let Some(first) = messages.next() {
                    let font = TextFont {
                        font_size: 24.0,
                        font: default_font.0.clone(),
                        ..Default::default()
                    };
                    p.spawn((
                        shown_hud_message,
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(125.0),
                            ..Default::default()
                        },
                        font.clone(),
                        Text::new(first.text),
                        TextColor(first.color),
                        TextLayout {
                            justify: JustifyText::Center,
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        for next_message in messages {
                            p.spawn((font.clone(), TextSpan::new(next_message.text), TextColor(next_message.color)));
                        }
                    });
                }
            });
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<HudMessages>().add_systems(
        Update,
        display_hud_messages
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
