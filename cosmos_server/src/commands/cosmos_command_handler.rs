use bevy::prelude::{App, EventReader, Res, ResMut};

use super::{CosmosCommandInfo, CosmosCommandSent, CosmosCommands};

fn register_commands(mut commands: ResMut<CosmosCommands>) {
    commands.add_command_info(CosmosCommandInfo {
        name: "help".into(),
        usage: "help [command?]".into(),
        description: "Gets information about every command.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "ping".into(),
        usage: "ping".into(),
        description: "Says 'Pong'.".into(),
    });

    commands.add_command_info(CosmosCommandInfo {
        name: "save".into(),
        usage: "save [entity_id] [file_name]".into(),
        description: "Saves the given structure to that file. Do not specify the file extension."
            .into(),
    });
}

fn display_help(command_name: Option<&str>, commands: &CosmosCommands) {
    if let Some(command_name) = command_name {
        if let Some(info) = commands.command_info(command_name) {
            println!("=== {} ===", info.name);
            println!("\t{}\n\t{}", info.usage, info.description);

            return;
        }
    }

    println!("=== All Commands ===");
    for (_, info) in commands.commands() {
        println!("{}\n\t{}\n\t{}", info.name, info.usage, info.description);
    }
}

fn cosmos_command_listener(
    mut command_events: EventReader<CosmosCommandSent>,
    commands: Res<CosmosCommands>,
) {
    for ev in command_events.iter() {
        match ev.name.as_str() {
            "help" => {
                if ev.args.len() != 1 {
                    display_help(None, &commands);
                } else {
                    display_help(Some(&ev.args[0]), &commands);
                }
            }
            "ping" => {
                println!("Pong");
            }
            "save" => {
                if ev.args.len() != 2 {
                    display_help(Some("save"), &commands);
                } else {
                    println!("SAVE {} to {}.cstr!!!", ev.args[0], ev.args[1]);
                }
            }
            _ => {
                display_help(Some(&ev.text), &commands);
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_startup_system(register_commands)
        .add_system(cosmos_command_listener);
}
