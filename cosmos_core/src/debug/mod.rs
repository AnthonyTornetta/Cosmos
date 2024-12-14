//! Useful debugging utilities used for both the client and the server

use bevy::{
    ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel},
    prelude::*,
    render::{pipelined_rendering::RenderExtractApp, Render, RenderApp},
};
use renet2::{RenetClient, RenetServer};

use crate::structure::Structure;

/// FIXME: bevy should not have any ambiguities, but it takes time to clean these up,
/// so we're juste ignoring those for now.
///
/// See [#7386](https://github.com/bevyengine/bevy/issues/7386) for relevant issue.
pub fn get_ignored_ambiguous_systems() -> Vec<Box<dyn ScheduleLabel>> {
    vec![
        Box::new(First),
        Box::new(PreUpdate),
        Box::new(PostUpdate),
        Box::new(Last),
        Box::new(ExtractSchedule),
        Box::new(Render),
    ]
}

fn configure_ambiguity_detection(sub_app: &mut SubApp) {
    let ignored_ambiguous_systems = get_ignored_ambiguous_systems();
    let mut schedules = sub_app.world_mut().resource_mut::<Schedules>();
    for (_, schedule) in schedules.iter_mut() {
        if ignored_ambiguous_systems.iter().any(|label| **label == *schedule.label()) {
            continue;
        }
        schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            use_shortnames: false,
            ..default()
        });
    }
}

// fn assert_no_conflicting_systems(sub_app: &SubApp) {
//     let ignored_ambiguous_systems = get_ignored_ambiguous_systems();

//     let schedules = sub_app.world().resource::<Schedules>();
//     for (_, schedule) in schedules.iter() {
//         if ignored_ambiguous_systems.iter().any(|label| **label == *schedule.label()) {
//             continue;
//         }
//         assert!(schedule.graph().conflicting_systems().is_empty());
//     }
// }

pub(super) fn register(app: &mut App) {
    // let sub_app = app.main_mut();
    // configure_ambiguity_detection(sub_app);
    // let sub_app = app.sub_app_mut(RenderApp);
    // configure_ambiguity_detection(sub_app);
    // let sub_app = app.sub_app_mut(RenderExtractApp);
    // configure_ambiguity_detection(sub_app);
    //
    // app.allow_ambiguous_resource::<RenetClient>();
    // app.allow_ambiguous_resource::<RenetServer>();
    //
    // app.allow_ambiguous_component::<Structure>();
    // app.allow_ambiguous_component::<Transform>();
    //
    // // app.finish();
    // // app.cleanup();
    // // app.update();
    //
    // fn assert_no_conflicting_systems(sub_app: &SubApp) {
    //     let ignored_ambiguous_systems = get_ignored_ambiguous_systems();
    //
    //     let schedules = sub_app.world().resource::<Schedules>();
    //     for (_, schedule) in schedules.iter() {
    //         if ignored_ambiguous_systems.iter().any(|label| **label == *schedule.label()) {
    //             continue;
    //         }
    //         assert!(schedule.graph().conflicting_systems().is_empty());
    //     }
    // }
    // let sub_app = app.main();
    // assert_no_conflicting_systems(sub_app);
}
