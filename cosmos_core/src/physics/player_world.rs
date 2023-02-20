use bevy::{
    prelude::{App, Component, Entity},
    reflect::{FromReflect, Reflect},
};

/// Represents a world of objects that are based around a certain entity
///
/// This entity must be:
/// - On the server, a player
/// - On the client, the local player
#[derive(Component, Reflect, FromReflect, Debug, Clone, Copy)]
pub struct PlayerWorld {
    pub player: Entity,
}

/// Represents the world this entity is within - make sure to double check this is still valid when using it
#[derive(Component, Reflect, FromReflect, Debug, Clone, Copy)]
pub struct WorldWithin(pub Entity);

pub(crate) fn register(app: &mut App) {
    app.register_type::<PlayerWorld>();
    app.register_type::<WorldWithin>();
}
