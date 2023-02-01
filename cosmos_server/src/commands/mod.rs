use std::time::Duration;

use bevy::{
    prelude::{App, EventWriter, Resource},
    utils::HashMap,
};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
pub mod cosmos_command_handler;

#[derive(Debug)]
pub struct CosmosCommandSent {
    /// The raw string the user typed
    pub text: String,
    /// The name of the command
    pub name: String,
    /// The args split around spaces
    pub args: Vec<String>,
}

impl CosmosCommandSent {
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
pub struct CosmosCommandInfo {
    pub name: String,
    pub usage: String,
    pub description: String,
}

#[derive(Resource, Debug, Default)]
pub struct CosmosCommands {
    commands: HashMap<String, CosmosCommandInfo>,
}

impl CosmosCommands {
    pub fn command_exists(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn command_info(&self, name: &str) -> Option<&CosmosCommandInfo> {
        self.commands.get(name)
    }

    pub fn add_command_info(&mut self, command_info: CosmosCommandInfo) {
        self.commands
            .insert(command_info.name.clone(), command_info);
    }

    pub fn remove_command_info(&mut self, name: &str) {
        self.commands.remove(name);
    }

    pub fn commands(&self) -> &HashMap<String, CosmosCommandInfo> {
        &self.commands
    }
}

fn monitor_inputs(mut event_writer: EventWriter<CosmosCommandSent>) {
    let mut text = String::default();

    while let Ok(event_available) = poll(Duration::ZERO) {
        if event_available {
            if let Ok(Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
                ..
            })) = read()
            {
                if kind != KeyEventKind::Release {
                    if let KeyCode::Char(mut c) = code {
                        if modifiers.intersects(KeyModifiers::SHIFT) {
                            c = c.to_uppercase().next().unwrap();
                        }

                        text.push(c);
                    }
                }
            }
        } else {
            break;
        }
    }

    if !text.trim().is_empty() {
        event_writer.send(CosmosCommandSent::new(text));
    }
}

pub(crate) fn register(app: &mut App) {
    app.insert_resource(CosmosCommands::default())
        .add_system(monitor_inputs)
        .add_event::<CosmosCommandSent>();

    cosmos_command_handler::register(app);
}
