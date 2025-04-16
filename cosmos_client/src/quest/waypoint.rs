use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{ecs::NeedsDespawned, netty::client::LocalPlayer, quest::OngoingQuests};

use crate::{
    ui::{UiSystemSet, ship_flight::indicators::IndicatorSettings},
    universe::map::waypoint::Waypoint,
};

use super::ActiveQuest;

#[derive(Component)]
struct ActiveQuestWaypoint;

fn on_active_quest(
    mut commands: Commands,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_active: Query<(&ActiveQuest, &OngoingQuests), (Changed<ActiveQuest>, With<LocalPlayer>)>,
    q_active_quest_waypoint: Query<Entity, With<ActiveQuestWaypoint>>,
    mut removed_components: RemovedComponents<ActiveQuest>,
) {
    for e in removed_components.read() {
        if !q_local_player.contains(e) {
            continue;
        }
        if let Ok(ent) = q_active_quest_waypoint.get_single() {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }

    for (aq, ongoing) in q_active.iter() {
        if let Ok(ent) = q_active_quest_waypoint.get_single() {
            commands.entity(ent).insert(NeedsDespawned);
        }

        let Some(q) = ongoing.from_id(&aq.0) else {
            continue;
        };

        let Some(loc) = q.details.location else {
            continue;
        };

        commands.spawn((
            Name::new("Quest Waypoint"),
            IndicatorSettings {
                color: css::AQUA.into(),
                max_distance: f32::INFINITY,
                offset: Vec3::ZERO,
            },
            loc,
            ActiveQuestWaypoint,
            Waypoint,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_active_quest.after(UiSystemSet::FinishUi));
}
