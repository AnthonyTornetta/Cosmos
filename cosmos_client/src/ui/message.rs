//! Displays any messages the player needs to see

use std::time::Duration;

use bevy::prelude::*;
use cosmos_core::state::GameState;

use super::font::DefaultFont;

const HUD_DISPLAY_DURATION: Duration = Duration::from_secs(7);
const FADE_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Reflect)]
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

#[derive(Debug, Clone, Reflect)]
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

    /// Iterates over all the richtext present in this hud message
    pub fn iter(&self) -> impl Iterator<Item = &'_ RichText> {
        self.text.iter()
    }
}

#[derive(Resource, Debug, Default)]
/// Used to interact with messages shown to the user in the above-hotbar dislay area.
pub struct HudMessages(Option<HudMessage>);

impl HudMessages {
    /// Adds this HUD message to the queue of messages to be displayed, and will display it when it is ready.
    ///
    /// Note that this will check for hud messages with the same text and prevent duplicate entries
    pub fn display_message(&mut self, message: HudMessage) {
        self.0 = Some(message);
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct ShownHudMessage {
    time_created: f32,
}

fn display_hud_messages(
    default_font: Res<DefaultFont>,
    mut commands: Commands,
    mut shown_hud_message: Query<(Entity, &ChildOf, &mut ShownHudMessage)>,
    mut hud_messages: ResMut<HudMessages>,
    mut writer: TextUiWriter,
    time: Res<Time>,
) {
    if let Ok((entity, parent, mut shown_hud_message)) = shown_hud_message.single_mut() {
        if hud_messages.0.is_some() {
            commands.entity(parent.parent()).despawn();
        } else {
            let time_now = time.elapsed_secs();

            let time_remaining = HUD_DISPLAY_DURATION.as_secs_f32() - (time_now - shown_hud_message.time_created);

            if time_remaining <= 0.0 {
                commands.entity(parent.parent()).despawn();
            } else {
                writer.for_each_color(entity, |mut c| c.set_alpha((time_remaining / FADE_DURATION.as_secs_f32()).min(1.0)));
            }
        }
    }

    if let Some(hud_message) = std::mem::take(&mut hud_messages.0) {
        let shown_hud_message = ShownHudMessage {
            time_created: time.elapsed_secs(),
        };

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
    app.init_resource::<HudMessages>()
        .add_systems(Update, display_hud_messages.run_if(in_state(GameState::Playing)));
}
