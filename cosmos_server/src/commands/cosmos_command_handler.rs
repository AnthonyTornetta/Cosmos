//! Handles all the server console commands

use std::time::Duration;

use bevy::{ecs::system::ScheduleSystem, prelude::*};
use cosmos_core::{
    chat::ServerSendChatMessageEvent,
    commands::ClientCommandEvent,
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
        system_sets::NetworkingSystemsSet,
    },
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, poll, read};
use thiserror::Error;

use crate::persistence::loading::LoadingSystemSet;

use super::{CommandSender, CosmosCommandSent, Operator, SendCommandMessageEvent, ServerCommand};

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
pub fn create_cosmos_command<T: CosmosCommandType, M>(
    command: ServerCommand,
    app: &mut App,
    on_get_command: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
    let unlocalized_name = command.unlocalized_name().to_owned();

    app.add_systems(OnEnter(GameState::Loading), move |mut reg: ResMut<Registry<ServerCommand>>| {
        reg.register(command.clone());
    });

    let monitor_commands = move |commands: Res<Registry<ServerCommand>>,
                                 mut evr_command_sent: EventReader<CosmosCommandSent>,
                                 mut evw_command: MessageWriter<CommandEvent<T>>,
                                 q_operator: Query<&Operator>,
                                 mut evw_send_message: MessageWriter<SendCommandMessageEvent>| {
        for ev in evr_command_sent.read() {
            if ev.name == unlocalized_name {
                if T::requires_operator() && !ev.sender.is_operator(&q_operator) {
                    ev.sender
                        .write("This command requires operator permissions.", &mut evw_send_message);
                    continue;
                }

                match T::from_input(ev) {
                    Ok(command) => {
                        evw_command.write(CommandEvent {
                            name: ev.name.clone(),
                            text: ev.text.clone(),
                            args: ev.args.clone(),
                            sender: ev.sender,
                            command,
                        });
                    }
                    Err(e) => {
                        ev.sender.write(format!("Command error: {e:?}"), &mut evw_send_message);
                        display_help(&ev.sender, &mut evw_send_message, Some(&ev.name), &commands);
                    }
                }
                continue;
            }
        }
    };

    app.add_systems(
        FixedUpdate,
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

    /// Returns true if this command requires operator permissions to use
    fn requires_operator() -> bool {
        true
    }
}

struct HelpCommand(Option<String>);
impl CosmosCommandType for HelpCommand {
    fn from_input(input_event: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if input_event.args.len() >= 2 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(Self(input_event.args.first().cloned()))
    }
}

fn register_commands(app: &mut App) {
    create_cosmos_command::<HelpCommand, _>(
        ServerCommand::new("cosmos:help", "[command?]", "Gets information about every command."),
        app,
        |mut evr_command: EventReader<CommandEvent<HelpCommand>>,
         commands: Res<Registry<ServerCommand>>,
         mut evw_send_message: MessageWriter<SendCommandMessageEvent>| {
            for ev in evr_command.read() {
                if let Some(cmd) = &ev.command.0 {
                    display_help(&ev.sender, &mut evw_send_message, Some(cmd.as_str()), &commands);
                } else {
                    display_help(&ev.sender, &mut evw_send_message, None, &commands);
                }
            }
        },
    );
}

fn display_help(
    sender: &CommandSender,
    evw_send_message: &mut MessageWriter<SendCommandMessageEvent>,
    command_name: Option<&str>,
    commands: &Registry<ServerCommand>,
) {
    if let Some(command_name) = command_name {
        let name = if !command_name.contains(":") {
            format!("cosmos:{command_name}")
        } else {
            command_name.into()
        };
        if let Some(info) = commands.from_id(&name) {
            sender.write(format!("=== {} ===", info.display_name()), evw_send_message);
            sender.write(
                format!("\t{} {} \t {}", info.display_name(), info.usage, info.description),
                evw_send_message,
            );

            return;
        }
    }

    sender.write("=== All Commands ===", evw_send_message);
    for command in commands.iter() {
        sender.write(command.display_name().to_string(), evw_send_message);
        if !command.usage.is_empty() {
            sender.write(format!("\t{}", command.usage), evw_send_message);
        }
        if !command.description.is_empty() {
            sender.write(format!("\t{}", command.description), evw_send_message);
        }
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

fn warn_on_no_command_hit(
    commands: Res<Registry<ServerCommand>>,
    mut evr_command: EventReader<CosmosCommandSent>,
    mut evw_send_message: MessageWriter<SendCommandMessageEvent>,
) {
    for ev in evr_command.read() {
        if !commands.contains(&ev.name) {
            ev.sender
                .write(format!("{} is not a recognized command.", ev.name), &mut evw_send_message);
            display_help(&ev.sender, &mut evw_send_message, None, &commands);
        }
    }
}

#[derive(Resource, Debug, Default)]
struct CurrentlyWriting(String);

fn monitor_inputs(mut event_writer: MessageWriter<CosmosCommandSent>, mut text: ResMut<CurrentlyWriting>) {
    while let Ok(event_available) = poll(Duration::ZERO) {
        if event_available {
            let x = read();

            if let Ok(crossterm::event::Event::Key(KeyEvent { code, modifiers, kind, .. })) = x
                && kind != KeyEventKind::Release
            {
                if let KeyCode::Char(mut c) = code {
                    if modifiers.intersects(KeyModifiers::SHIFT) {
                        c = c.to_uppercase().next().unwrap();
                    }

                    text.0.push(c);
                } else if KeyCode::Enter == code {
                    text.0.push('\n');
                }
            }
        } else {
            break;
        }
    }

    if !text.0.trim().is_empty() && text.0.ends_with('\n') {
        let cmd = CosmosCommandSent::new(text.0[0..text.0.len() - 1].to_owned(), CommandSender::Server);
        event_writer.write(cmd);

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

fn command_receiver(
    mut event_writer: MessageWriter<CosmosCommandSent>,
    mut nevr_command: EventReader<NettyMessageReceived<ClientCommandEvent>>,
    q_player: Query<&Player>,
    lobby: Res<ServerLobby>,
) {
    for client_command in nevr_command.read() {
        let Some(player) = lobby.player_from_id(client_command.client_id) else {
            continue;
        };

        let Ok(p) = q_player.get(player) else {
            continue;
        };

        info!("Player `{}` ran command: `{}`", p.name(), client_command.command_text);
        event_writer.write(CosmosCommandSent::new(
            client_command.command_text.clone(),
            CommandSender::Player(player),
        ));
    }
}

fn send_messages(
    mut evw_chat_event: NettyMessageWriter<ServerSendChatMessageEvent>,
    mut evr_send_message: EventReader<SendCommandMessageEvent>,
    q_player: Query<&Player>,
) {
    for ev in evr_send_message.read() {
        let Ok(player) = q_player.get(ev.to) else {
            continue;
        };

        info!("({}) {}", player.name(), ev.message);
        evw_chat_event.write(
            ServerSendChatMessageEvent {
                sender: None,
                message: ev.message.clone(),
            },
            player.client_id(),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (ProcessCommandsSet::ParseCommands, ProcessCommandsSet::HandleCommands)
            .chain()
            .before(LoadingSystemSet::BeginLoading),
    );

    register_commands(app);
    app.insert_resource(CurrentlyWriting::default())
        .add_systems(Update, monitor_inputs)
        .add_systems(
            FixedUpdate,
            (
                (command_receiver, warn_on_no_command_hit)
                    .chain()
                    .in_set(FixedUpdateSet::Main)
                    .in_set(ProcessCommandsSet::ParseCommands),
                send_messages
                    .after(ProcessCommandsSet::HandleCommands)
                    .before(NetworkingSystemsSet::SyncComponents),
            ),
        );
}
