//! Responsible for building ships for the client.

use bevy::{
    ecs::system::EntityCommands,
    prelude::{
        resource_exists, Added, App, BuildChildren, Commands, Entity, Handle, IntoSystemConfigs, Name, Query, Res, Resource, Transform,
        Update,
    },
    transform::bundles::TransformBundle,
};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    physics::location::Location,
    structure::{
        loading::StructureLoadingSet,
        shared::DespawnWithStructure,
        ship::{
            ship_builder::{ShipBuilder, TShipBuilder},
            Ship,
        },
        Structure,
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter},
    state::game_state::GameState,
    structure::{chunk_retreiver::NeedsPopulated, client_structure_builder::ClientStructureBuilder},
};

/// Responsible for building ships for the client.
pub struct ClientShipBuilder {
    ship_bulder: ShipBuilder<ClientStructureBuilder>,
}

impl Default for ClientShipBuilder {
    fn default() -> Self {
        Self {
            ship_bulder: ShipBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TShipBuilder for ClientShipBuilder {
    fn insert_ship(&self, entity: &mut EntityCommands, location: Location, velocity: Velocity, structure: &mut Structure) {
        self.ship_bulder.insert_ship(entity, location, velocity, structure);
    }
}

fn client_on_add_ship(
    query: Query<Entity, Added<Ship>>,
    engine_idle_sound: Res<EngineIdleSound>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        let playing_sound: Handle<AudioInstance> = audio.play(engine_idle_sound.0.clone()).with_volume(0.0).looped().handle();

        let idle_emitter = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 20.0,
            peak_volume: 0.15,
            ..Default::default()
        }]);

        commands.entity(entity).insert(NeedsPopulated).with_children(|p| {
            p.spawn((
                Name::new("Engine idle sound"),
                DespawnWithStructure,
                TransformBundle::from_transform(Transform::from_xyz(0.5, 0.5, 0.5)),
                idle_emitter,
            ));
        });
    }
}

#[derive(Resource)]
struct EngineIdleSound(Handle<AudioSource>);

struct LoadingShipAudio;

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, LoadingShipAudio>(
        app,
        GameState::PreLoading,
        vec!["cosmos/sounds/sfx/engine-idle.ogg"],
        |mut commands, mut handles| {
            commands.insert_resource(EngineIdleSound(handles.remove(0).0));
        },
    );

    app.add_systems(
        Update,
        client_on_add_ship
            .in_set(StructureLoadingSet::AddStructureComponents)
            .run_if(resource_exists::<EngineIdleSound>),
    );
}
