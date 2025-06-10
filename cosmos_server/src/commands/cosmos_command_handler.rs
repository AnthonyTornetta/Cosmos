//! Handles all the server console commands

use std::time::Duration;

use bevy::{
    app::Update,
    ecs::schedule::IntoSystemConfigs,
    log::{error, info},
    prelude::{App, Event, EventReader, EventWriter, IntoSystemSetConfigs, OnEnter, Res, ResMut, Resource, SystemSet, on_event},
};
use cosmos_core::{
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};
use thiserror::Error;

use crate::persistence::loading::LoadingSystemSet;

use super::{CommandSender, CosmosCommandSent, ServerCommand};

#[derive(Event, Debug)]
/// An event that is sent when this command for `T` is sent
///
/// Used with [`create_cosmos_command`]
pub struct CommandEvent<T> {
    /// The entity that sent this - None if this is called from the server console.
    pub sender: CommandSender,
    /// The raw string the user typed
    pub text: String,
    /// The name of the command (as the user typed - may be missing the `cosmos:` identifier)
    pub name: String,
    /// The args split around spaces
    pub args: Vec<String>,
    /// The command generated from the [`CosmosCommandType::from_input`]
    pub command: T,
}

/// Used to easily create your own cosmos command.
///
/// The system passed will be called when a [`CommandEvent<T>`] for your `T` is generated. You will
/// still need to read them via a normal [`EventReader<CommandEvent<T>>`] in your system.
pub fn create_cosmos_command<T: CosmosCommandType, M>(command: ServerCommand, app: &mut App, on_get_command: impl IntoSystemConfigs<M>) {
    let unlocalized_name = command.unlocalized_name().to_owned();

    app.add_systems(OnEnter(GameState::Loading), move |mut reg: ResMut<Registry<ServerCommand>>| {
        reg.register(command.clone());
    });

    let monitor_commands = move |commands: Res<Registry<ServerCommand>>,
                                 mut evr_command_sent: EventReader<CosmosCommandSent>,
                                 mut evw_command: EventWriter<CommandEvent<T>>| {
        for ev in evr_command_sent.read() {
            if ev.name == unlocalized_name {
                match T::from_input(ev) {
                    Ok(command) => {
                        evw_command.send(CommandEvent {
                            name: ev.name.clone(),
                            text: ev.text.clone(),
                            args: ev.args.clone(),
                            sender: ev.sender.clone(),
                            command,
                        });
                    }
                    Err(e) => {
                        error!("Command error: {e:?}");
                        display_help(Some(&ev.name), &commands);
                    }
                }
                continue;
            }
        }
    };

    app.add_systems(
        Update,
        (monitor_commands, on_get_command.run_if(on_event::<CommandEvent<T>>))
            .in_set(ProcessCommandsSet::HandleCommands)
            .chain(),
    )
    .add_event::<CommandEvent<T>>();
}

/// A cosmos command event type
pub trait CosmosCommandType: Sized + Send + Sync + 'static {
    /// Parses the raw command input into your command or an [`ArgumentError`].
    fn from_input(input_event: &CosmosCommandSent) -> Result<Self, ArgumentError>;
}

struct HelpCommand(Option<String>);
impl CosmosCommandType for HelpCommand {
    fn from_input(input_event: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if input_event.args.len() >= 2 {
            return Err(ArgumentError::TooManyArguments);
        }

        return Ok(Self(input_event.args.get(0).cloned()));
    }
}

fn register_commands(app: &mut App) {
    create_cosmos_command::<HelpCommand, _>(
        ServerCommand::new("cosmos:help", "[command?]", "Gets information about every command."),
        app,
        |mut evr_command: EventReader<CommandEvent<HelpCommand>>, commands: Res<Registry<ServerCommand>>| {
            for ev in evr_command.read() {
                if let Some(cmd) = &ev.command.0 {
                    display_help(Some(cmd.as_str()), &commands);
                } else {
                    display_help(None, &commands);
                }
            }
        },
    );
}

fn display_help(command_name: Option<&str>, commands: &Registry<ServerCommand>) {
    if let Some(command_name) = command_name {
        let name = if !command_name.contains(":") {
            format!("cosmos:{command_name}")
        } else {
            command_name.into()
        };
        if let Some(info) = commands.from_id(&name) {
            println!("=== {} ===", info.display_name());
            println!("\t{} {}\n\t{}", info.display_name(), info.usage, info.description);

            return;
        }
    }

    println!("=== All Commands ===");
    for command in commands.iter() {
        println!("{}\n\t{}\n\t{}", command.display_name(), command.usage, command.description);
    }
}

#[derive(Debug, Error)]
/// Something was wrong with the arguments in the command
pub enum ArgumentError {
    /// Too few arguments
    #[error("Too few arguments")]
    TooFewArguments,
    /// Too many arguments
    #[error("Too many arguments")]
    TooManyArguments,
    /// One of the types was invalid
    #[error("Invalid type at {arg_index} - wanted {type_name}")]
    InvalidType {
        /// The index in the arguments list that was wrong
        arg_index: u32,
        /// What the type should have been (ie `u16`, `Entity`).
        type_name: String,
    },
}

fn warn_on_no_command_hit(commands: Res<Registry<ServerCommand>>, mut evr_command: EventReader<CosmosCommandSent>) {
    for ev in evr_command.read() {
        if !commands.contains(&ev.name) {
            info!("{} is not a recognized command.", ev.name);
            display_help(None, &commands);
        }
    }
}

#[derive(Resource, Debug, Default)]
struct CurrentlyWriting(String);

fn monitor_inputs(mut event_writer: EventWriter<CosmosCommandSent>, mut text: ResMut<CurrentlyWriting>) {
    while let Ok(event_available) = poll(Duration::ZERO) {
        if event_available {
            let x = read();

            if let Ok(crossterm::event::Event::Key(KeyEvent { code, modifiers, kind, .. })) = x {
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
        let cmd = CosmosCommandSent::new(text.0[0..text.0.len() - 1].to_owned(), CommandSender::Server);
        event_writer.send(cmd);

        text.0.clear();
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The set in which commands are processed
pub enum ProcessCommandsSet {
    /// User input is parsed and events are sent
    ParseCommands,
    /// Commands should be handled and command events read from
    HandleCommands,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (ProcessCommandsSet::ParseCommands, ProcessCommandsSet::HandleCommands)
            .chain()
            .before(LoadingSystemSet::BeginLoading),
    );

    register_commands(app);
    app.insert_resource(CurrentlyWriting::default()).add_systems(
        Update,
        (monitor_inputs, warn_on_no_command_hit)
            .chain()
            .in_set(ProcessCommandsSet::ParseCommands),
    );
}
