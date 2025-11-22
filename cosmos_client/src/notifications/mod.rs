//! Client-side notification processing

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::notifications::{Notification, NotificationKind};

use crate::ui::message::{HudMessage, HudMessages};

fn on_recv_notification(mut hud_msgs: ResMut<HudMessages>, mut evr_notification: MessageReader<Notification>) {
    for not in evr_notification.read() {
        hud_msgs.display_message(HudMessage::with_colored_string(
            not.message(),
            match not.kind() {
                NotificationKind::Error => css::RED.into(),
                NotificationKind::Info => css::AQUA.into(),
            },
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_recv_notification);
}
