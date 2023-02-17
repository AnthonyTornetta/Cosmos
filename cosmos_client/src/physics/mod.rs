use bevy::prelude::{App, GlobalTransform, Parent, Query, Transform, With, Without};

use crate::netty::flags::LocalPlayer;

// fn move_everything(
//     mut query_local_transform: Query<(&mut Transform, &GlobalTransform), With<LocalPlayer>>,
//     mut everything_else_transform: Query<&mut Transform, (Without<Parent>, Without<LocalPlayer>)>,
// ) {
//     if let Ok((mut player_transform, global_transform)) = query_local_transform.get_single_mut() {
//         for mut trans in everything_else_transform.iter_mut() {
//             trans.translation -= global_transform.translation();
//         }

//         player_transform.translation.x = 0.0;
//         player_transform.translation.y = 0.0;
//         player_transform.translation.z = 0.0;
//     }
// }

pub(crate) fn register(app: &mut App) {
    // app.add_system(move_everything);
}
