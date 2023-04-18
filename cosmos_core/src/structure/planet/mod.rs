//! A planet is a structure that does not move and emits gravity.
//!
//! These are not made by the player but generated

use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

pub mod planet_builder;

#[derive(Component, Debug, Reflect, FromReflect)]
/// If a structure has this, it is a planet.
pub struct Planet;
