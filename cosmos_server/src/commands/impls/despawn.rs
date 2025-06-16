use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::ecs::NeedsDespawned;

struct DespawnCommand(Entity);

impl CosmosCommandType for DespawnCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        if let Ok(index) = ev.args[0].parse::<u64>() {
            if let Ok(entity) = Entity::try_from_bits(index) {
                Ok(DespawnCommand(entity))
            } else {
                Err(ArgumentError::InvalidType {
                    arg_index: 0,
                    type_name: "Entity".into(),
                })
            }
        } else {
            Err(ArgumentError::InvalidType {
                arg_index: 0,
                type_name: "Entity".into(),
            })
        }
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<DespawnCommand, _>(
        ServerCommand::new("cosmos:despawn", "[entity_id]", "Despawns the given entity."),
        app,
        |mut commands: Commands, mut evr_command: EventReader<CommandEvent<DespawnCommand>>| {
            for ev in evr_command.read() {
                if let Ok(mut entity_commands) = commands.get_entity(ev.command.0) {
                    entity_commands.insert(NeedsDespawned);
                    println!("Despawned entity {:?}", ev.command.0);
                } else {
                    println!("Entity not found");
                }
            }
        },
    );
}
