//! Responsible for the default generation of biospheres.

use std::fs;

use crate::netty::connect::WaitingOnServer;
use bevy::prelude::*;
use bevy_easy_compute::prelude::*;
use cosmos_core::{
    ecs::{add_multi_statebound_resource, add_statebound_resource, init_resource},
    netty::system_sets::NetworkingSystemsSet,
    state::GameState,
    structure::planet::generation::terrain_generation::{
        add_terrain_compute_worker, BiosphereShaderWorker, ChunkData, GpuPermutationTable,
    },
};

#[derive(Event, Debug)]
/// Sent whenever the terrain generation data is updated from the server
pub(crate) struct SetTerrainGenData {
    /// The files for wgsl shaders (path, shader code)
    pub files: Vec<(String, String)>,
    /// The permutation table to send to the GPU
    pub permutation_table: GpuPermutationTable,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Stages a biosphere must go through to generate a chunk
pub enum BiosphereGenerationSet {
    /// The biosphere should flag the chunks they want generated by adding them to the [`NeedGeneratedChunks`] resource.
    FlagChunksNeedGenerated,
    /// Chunk generation requests are sent to the GPU when it is available for new generations. This is handled for all biospheres
    /// automatically that put their chunk requests in [`NeedGeneratedChunks`]
    GpuInteraction,
    /// Chunks that are ready to be populated with blocks are now sent and can be read via the EventReader for [`DoneGeneratingChunkEvent`].
    GenerateChunks,
    /// Called after the [`BiosphereGenerationSet::GenerateChunks`] set. This should be used for things like trees.
    GenerateChunkFeatures,
}

#[derive(Resource)]
struct NeedsTerrainDataFlag(Entity);

fn add_needs_terrain_data(mut commands: Commands) {
    let entity = commands
        .spawn((Name::new("Waiting on biosphere compute shader + values"), WaitingOnServer))
        .id();

    commands.insert_resource(NeedsTerrainDataFlag(entity));
}

#[derive(Resource, Default)]
struct SetPermutationTable(GpuPermutationTable);

fn setup_lod_generation(
    mut commands: Commands,
    mut ev_reader: EventReader<SetTerrainGenData>,
    terrain_data_flag: Res<NeedsTerrainDataFlag>,
    mut gpu_perm_table: ResMut<SetPermutationTable>,
) {
    for ev in ev_reader.read() {
        let mut working_dir = std::env::current_dir().expect("Can't get working dir");
        working_dir.push("./assets/temp/shaders/biosphere/");

        // Clears out any existing shaders from previous servers
        let _ = fs::remove_dir_all(&working_dir);

        for (file_name, file_contents) in ev.files.iter() {
            if file_name.contains("..") || !file_name.ends_with(".wgsl") {
                error!("File name '{file_name}' contained '..' or didn't end in '.wgsl' - this file will not be created!");
                continue;
            }

            let mut path_buf = working_dir.clone();
            path_buf.push(file_name);

            if !path_buf.as_path().starts_with(&working_dir) {
                error!("The path traversed outside of the biosphere shaders directory - not saving file.");
                continue;
            }

            let dir = path_buf.parent().expect("Path has no directory? This should be impossible.");
            // This can fail if it's already there
            let _ = fs::create_dir_all(dir);

            let file_contents = file_contents.replacen("#import \"", "#import \"temp/shaders/biosphere/", usize::MAX);

            if let Err(e) = fs::write(path_buf, file_contents) {
                error!("{:?}", e);
                continue;
            }
        }

        gpu_perm_table.0 = ev.permutation_table.clone();

        commands.insert_resource(ev.permutation_table.clone());
        commands.remove_resource::<NeedsTerrainDataFlag>();
        commands.entity(terrain_data_flag.0).despawn_recursive();
    }
}

fn send_permutation_table_to_worker(
    mut commands: Commands,
    mut worker: ResMut<AppComputeWorker<BiosphereShaderWorker>>,
    permutation_table: Res<SetPermutationTable>,
) {
    worker.write_slice("permutation_table", &permutation_table.0 .0);

    commands.remove_resource::<SetPermutationTable>();
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            BiosphereGenerationSet::FlagChunksNeedGenerated,
            BiosphereGenerationSet::GpuInteraction,
            BiosphereGenerationSet::GenerateChunks,
            BiosphereGenerationSet::GenerateChunkFeatures,
        )
            .chain(),
    )
    .add_plugins(AppComputeWorkerPlugin::<BiosphereShaderWorker>::default())
    .add_systems(OnExit(GameState::LoadingWorld), add_terrain_compute_worker)
    .add_systems(OnEnter(GameState::LoadingWorld), add_needs_terrain_data)
    .add_systems(
        Update,
        setup_lod_generation
            .run_if(resource_exists::<NeedsTerrainDataFlag>)
            .in_set(NetworkingSystemsSet::ProcessReceivedMessages)
            .run_if(in_state(GameState::LoadingWorld)),
    )
    .add_systems(
        OnEnter(GameState::Playing),
        send_permutation_table_to_worker.after(init_resource::<SetPermutationTable>),
    )
    .add_event::<SetTerrainGenData>();

    add_multi_statebound_resource::<SetPermutationTable, GameState>(app, GameState::LoadingData, GameState::Playing);
    add_statebound_resource::<ChunkData, GameState>(app, GameState::Playing);
}
