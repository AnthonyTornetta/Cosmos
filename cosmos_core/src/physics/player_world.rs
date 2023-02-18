use bevy::{
    prelude::{Component, Entity},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect, Debug)]
pub struct PlayerWorld(pub Entity);
