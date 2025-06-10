//! Responsible for the registration & creation elements of all server console commands

use bevy::prelude::{App, Entity, Event};
use cosmos_core::registry::{create_registry, identifiable::Identifiable};

pub mod cosmos_command_handler;
mod impls;
pub mod prelude;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The entity or server that sent this command
pub enum CommandSender {
    /// The server sent this command
    Server,
    /// A player sent this command
    Player(Entity),
}

#[derive(Debug, Event)]
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

    app.add_event::<CosmosCommandSent>();

    cosmos_command_handler::register(app);
    impls::register(app);
}
