use bevy::prelude::*;
use cosmos_core::{persistence::Blueprintable, physics::location::Location};

use crate::persistence::saving::NeedsBlueprinted;

use super::super::prelude::*;

struct BlueprintCommand(Entity);

impl CosmosCommandType for BlueprintCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() < 2 {
            return Err(ArgumentError::TooFewArguments);
        } else if ev.args.len() > 2 {
            return Err(ArgumentError::TooManyArguments);
        }

        let Ok(index) = ev.args[0].parse::<u64>() else {
            return Err(ArgumentError::InvalidType {
                arg_index: 0,
                type_name: "u64".into(),
            });
        };

        let Ok(entity) = Entity::try_from_bits(index) else {
            return Err(ArgumentError::InvalidType {
                arg_index: 0,
                type_name: "Entity".into(),
            });
        };

        Ok(BlueprintCommand(entity))
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<BlueprintCommand, _>(
        ServerCommand::new(
            "cosmos:blueprint",
            "[entity_id] [file_name]",
            "blueprints the given structure to that file. Do not specify the file extension.",
        ),
        app,
        |mut evr_blueprint: EventReader<CommandEvent<BlueprintCommand>>,
         all_blueprintable_entities: Query<(Entity, &Name, &Location), With<Blueprintable>>,
         mut commands: Commands| {
            for ev in evr_blueprint.read() {
                if !all_blueprintable_entities.contains(ev.command.0) {
                    warn!("This entity is not blueprintable!");
                    continue;
                };

                info!("Blueprinting entity!");

                commands.entity(ev.command.0).insert(NeedsBlueprinted {
                    blueprint_name: ev.args[1].to_owned(),
                    ..Default::default()
                });
            }
        },
    );
}
