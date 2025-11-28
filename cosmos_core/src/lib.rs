//! The core package contains common functionality that is shared between the client & the server.

#![feature(duration_constructors)]
#![warn(missing_docs)]
// This one has a stupid rule where if you have `fn (&self) -> HasLifetime`, you need to do `fn (&self) -> HasLifetime<'_>`. This is stupid.
#![allow(mismatched_lifetime_syntaxes)]

#[cfg(all(feature = "client", feature = "server", feature = "extra-build-checks"))]
compile_error!("You cannot enable both client and server features");

#[cfg(not(feature = "client"))]
#[cfg(not(feature = "server"))]
compile_error!("You cannot have both client and server features disabled");

pub mod block;
pub mod blockitems;
pub mod chat;
pub mod commands;
pub mod coms;
pub mod crafting;
pub mod creative;
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
pub mod notifications;
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
