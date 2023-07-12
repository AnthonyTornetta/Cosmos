//! Responsible for the registration & creation elements of all server console commands

use std::time::Duration;

use bevy::{
    prelude::{App, EventWriter, ResMut, Resource},
    reflect::{FromReflect, Reflect},
    utils::HashMap,
};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
pub mod cosmos_command_handler;

#[derive(Debug)]
/// This event is sent when the server admin types a console command
pub struct CosmosCommandSent {
    /// The raw string the user typed
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
    pub fn new(text: String) -> Self {
        let split: Vec<&str> = text.split(' ').collect();
        let (name_arr, args_arr) = split.split_at(1);

        let name = name_arr[0].to_lowercase();
        let args = args_arr
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| (*x).to_owned())
            .collect::<Vec<String>>();

        Self { text, name, args }
    }
}

#[derive(Debug)]
/// Information that describes how a command should be formatted by the user
pub struct CosmosCommandInfo {
    /// Name of the command.
    ///
    /// Example: "despawn"
    pub name: String,
    /// How to use the command.
    ///
    /// Example: "despawn [entity_id]"
    pub usage: String,
    /// What the command does.
    ///
    /// Example: "Despawns the entity with the given entity id."
    pub description: String,
}

#[derive(Resource, Debug, Default)]
/// This resource contains all the registered commands
///
/// This should eventually be replaced by a `Registry<CosmosCommandInfo>`
pub struct CosmosCommands {
    commands: HashMap<String, CosmosCommandInfo>,
}

impl CosmosCommands {
    /// Returns true if a command with that name exists
    pub fn command_exists(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Gets the information for this command, if it exists
    pub fn command_info(&self, name: &str) -> Option<&CosmosCommandInfo> {
        self.commands.get(name)
    }

    /// Adds information for a command, based on the `command_info` argument's name
    pub fn add_command_info(&mut self, command_info: CosmosCommandInfo) {
        self.commands.insert(command_info.name.clone(), command_info);
    }

    /// Removes the command info for the given command
    pub fn remove_command_info(&mut self, name: &str) {
        self.commands.remove(name);
    }

    /// Gets all the commands
    ///
    /// this is subject to removal in the future
    pub fn commands(&self) -> &HashMap<String, CosmosCommandInfo> {
        &self.commands
    }
}

#[derive(Resource, Reflect, FromReflect, Debug, Default)]
struct CurrentlyWriting(String);

fn monitor_inputs(mut event_writer: EventWriter<CosmosCommandSent>, mut text: ResMut<CurrentlyWriting>) {
    while let Ok(event_available) = poll(Duration::ZERO) {
        if event_available {
            if let Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) = read() {
                if kind != KeyEventKind::Release {
                    if let KeyCode::Char(mut c) = code {
                        if modifiers.intersects(KeyModifiers::SHIFT) {
                            c = c.to_uppercase().next().unwrap();
                        }

                        text.0.push(c);
                    } else if KeyCode::Enter == code {
                        text.0.push('\n');
                    }
                }
            }
        } else {
            break;
        }
    }

    if !text.0.trim().is_empty() && text.0.ends_with('\n') {
        event_writer.send(CosmosCommandSent::new(text.0[0..text.0.len() - 1].to_owned()));

        text.0.clear();
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CosmosCommands::default())
        .insert_resource(CurrentlyWriting::default())
        .add_system(monitor_inputs)
        .add_event::<CosmosCommandSent>();

    cosmos_command_handler::register(app);
}
