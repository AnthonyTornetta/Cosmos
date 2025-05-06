//! The core package contains common functionality that is shared between the client & the server.

#![feature(duration_constructors)]
#![warn(missing_docs)]

pub mod block;
pub mod blockitems;
pub mod chat;
pub mod coms;
pub mod crafting;
pub mod debug;
pub mod economy;
pub mod ecs;
pub mod entities;
pub mod events;
pub mod faction;
pub mod fluid;
pub mod inventory;
pub mod item;
pub mod loader;
pub mod logic;
pub mod netty;
pub mod persistence;
pub mod physics;
pub mod plugin;
pub mod prelude;
pub mod projectiles;
pub mod quest;
pub mod registry;
pub mod shop;
pub mod state;
pub mod structure;
pub mod time;
pub mod universe;
pub mod utils;
