//! Responsible for the registration & creation elements of all server console commands

use bevy::prelude::*;
use cosmos_core::{
    netty::sync::IdentifiableComponent,
    registry::{create_registry, identifiable::Identifiable},
};
use renet::ClientId;
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::DefaultPersistentComponent;

pub mod cosmos_command_handler;
mod impls;
mod operator;
mod parser;
pub mod prelude;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ServerOperator {
    /// This name field is just to easily identify people in the operators.json. This is NOT used
    /// for any actual logic
    name: String,
    steam_id: ClientId,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, Resource)]
/// A list of all operators a server has (includes logged out players)
///
/// An operator can execute server commands, and has the highest level of permissions
pub struct Operators(Vec<ServerOperator>);

impl Operators {
    /// Checks if a given client id is an operator
    pub fn is_operator(&self, steam_id: ClientId) -> bool {
        self.0.iter().any(|x| x.steam_id == steam_id)
    }

    /// Adds a player to the list of server operators
    pub fn add_operator(&mut self, steam_id: ClientId, name: impl Into<String>) {
        if let Some(existing) = self.0.iter_mut().find(|x| x.steam_id == steam_id) {
            existing.name = name.into();
        } else {
            self.0.push(ServerOperator {
                steam_id,
                name: name.into(),
            })
        }
    }

    /// Removes a player from the list of server operators
    pub fn remove_operator(&mut self, steam_id: ClientId) {
        self.0.retain(|x| x.steam_id != steam_id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The entity or server that sent this command
pub enum CommandSender {
    /// The server sent this command
    Server,
    /// A player sent this command
    Player(Entity),
}

impl CommandSender {
    /// Returns the entity for this command sender if it didn't come from the console
    pub fn entity(&self) -> Option<Entity> {
        match self {
            Self::Server => None,
            Self::Player(e) => Some(*e),
        }
    }
}

#[derive(Component, Debug, Serialize, Deserialize)]
/// If a player is an operator, they have all permissions
pub struct Operator;

impl IdentifiableComponent for Operator {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:operator"
    }
}

impl DefaultPersistentComponent for Operator {}

#[derive(Message, Debug)]
/// Sends output from a command to the player entity
pub struct SendCommandMessageMessage {
    to: Entity,
    message: String,
}

impl CommandSender {
    /// Checks if this sender is a server operator
    pub fn is_operator(&self, q_operator: &Query<&Operator>) -> bool {
        match self {
            Self::Player(e) => q_operator.contains(*e),
            Self::Server => true,
        }
    }

    /// Sends a message to this command sender
    ///
    /// Player - logged in chat and logged in server console
    /// Server - logged in server console
    pub fn write(&self, message: impl Into<String>, evw_send_message: &mut MessageWriter<SendCommandMessageMessage>) {
        match self {
            Self::Player(e) => {
                evw_send_message.write(SendCommandMessageMessage {
                    message: message.into(),
                    to: *e,
                });
            }
            Self::Server => {
                println!("{}", message.into());
            }
        }
    }
}

#[derive(Debug, Message)]
/// This event is sent when the server admin types a console command
pub struct CosmosCommandSent {
    /// The sender of this command - None if the server sent it
    pub sender: CommandSender,
    /// The raw string the user typed (this includes the command name)
    pub text: String,
    /// The name of the command
    pub name: String,
    /// The args split around spaces
    pub args: Vec<String>,
}

impl CosmosCommandSent {
    /// Creates a new command event.
    ///
    /// * `text` The entire string of text the user typed
    pub fn new(text: String, sender: CommandSender) -> Self {
        let split: Vec<&str> = text.split(' ').collect();
        let (name_arr, args_arr) = split.split_at(1);

        let mut name = name_arr[0].to_lowercase();
        if !name.contains(":") {
            name = format!("cosmos:{name}");
        }
        let args = args_arr
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| (*x).to_owned())
            .collect::<Vec<String>>();

        Self { text, name, args, sender }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Information that describes how a command should be formatted by the user
pub struct ServerCommand {
    id: u16,
    /// Name of the command.
    ///
    /// Example: "cosmos:despawn"
    pub unlocalized_name: String,
    /// How to use the command.
    ///
    /// Example: "\[entity_id\]"
    pub usage: String,
    /// What the command does.
    ///
    /// Example: "Despawns the entity with the given entity id."
    pub description: String,
}

impl Identifiable for ServerCommand {
    fn id(&self) -> u16 {
        self.id
    }
    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl ServerCommand {
    /// Creates a new cosmos command with these identifiers
    ///
    /// * `unlocalized_name` Used to call the command (modid:command_name)
    /// * `usage` Shows the usage - do not include the `unlocalized_name` in this
    /// * `description` An overview of what the command does
    pub fn new(unlocalized_name: impl Into<String>, usage: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: 0,
            usage: usage.into(),
            description: description.into(),
            unlocalized_name: unlocalized_name.into(),
        }
    }

    /// Returns how the command name should be displayed
    pub fn display_name(&self) -> String {
        if self.unlocalized_name().starts_with("cosmos:") {
            self.unlocalized_name()["cosmos:".len()..].to_owned()
        } else {
            self.unlocalized_name().to_owned()
        }
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<ServerCommand>(app, "cosmos:commands");

    app.add_message::<CosmosCommandSent>().add_message::<SendCommandMessageMessage>();

    cosmos_command_handler::register(app);
    impls::register(app);
    operator::register(app);
}
