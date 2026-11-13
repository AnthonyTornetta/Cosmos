use bevy::prelude::*;

pub(super) const BACKGROUND: Color = Color::srgb(0.067, 0.106, 0.145);
pub(super) const SURFACE: Color = Color::srgb(0.098, 0.149, 0.196);
pub(super) const BORDER: Color = Color::srgb(0.204, 0.282, 0.345);
pub(super) const SELECTED: Color = Color::srgb(0.125, 0.235, 0.278);
pub(super) const TEXT: Color = Color::srgb(0.890, 0.929, 0.949);
pub(super) const MUTED: Color = Color::srgb(0.635, 0.718, 0.776);
pub(super) const ACCENT: Color = Color::srgb(0.478, 0.827, 0.863);
pub(super) const DANGER: Color = Color::srgb(0.890, 0.627, 0.549);

pub(super) fn font(font: &Handle<Font>, size: f32) -> TextFont {
    TextFont {
        font: font.clone().into(),
        font_size: FontSize::Px(size),
        ..Default::default()
    }
}

pub(super) fn label(p: &mut ChildSpawnerCommands, handle: &Handle<Font>, text: &str, size: f32, color: Color) {
    p.spawn((
        Text::new(text),
        font(handle, size),
        TextColor(color),
        Node {
            max_width: Val::Percent(100.0),
            ..Default::default()
        },
    ));
}
