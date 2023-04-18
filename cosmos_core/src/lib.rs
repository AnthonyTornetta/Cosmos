//! The core package contains common functionality that is shared between the client & the server.

#![feature(get_many_mut)]
#![warn(missing_docs)]

pub mod block;
pub mod blockitems;
pub mod entities;
pub mod events;
pub mod inventory;
pub mod item;
pub mod loader;
pub mod netty;
pub mod physics;
pub mod plugin;
pub mod projectiles;
pub mod registry;
pub mod sector;
pub mod structure;
pub mod utils;
pub mod universe;