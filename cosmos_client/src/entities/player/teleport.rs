use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::player::teleport::TeleportMessage,
    netty::{client::LocalPlayer, netty_rigidbody::NettyRigidBodyLocation},
    physics::location::SetPosition,
};

fn on_teleport(q_loc: Query<Entity, With<LocalPlayer>>, mut nmr: MessageReader<TeleportMessage>, mut commands: Commands) {
    let Some(msg) = nmr.read().last() else {
        return;
    };

    let Ok(player_ent) = q_loc.single() else {
        return;
    };

    match msg.to {
        NettyRigidBodyLocation::Absolute(loc) => {
            commands.entity(player_ent).insert((SetPosition::Transform, loc));
        }
        NettyRigidBodyLocation::Relative(offset, entity) => {
            commands.entity(player_ent).insert(SetPosition::RelativeTo { entity, offset });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_teleport.in_set(FixedUpdateSet::Main));
}
