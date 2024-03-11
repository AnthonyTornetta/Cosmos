#import "cosmos/shaders/generation_utils.wgsl"::{
    GenerationParams,
    TerrainData,
    expand,
};

#import "cosmos/shaders/grass_biosphere.wgsl"::{
    generate
};

const N_CHUNKS: u32 = 32;
const CHUNK_DIMENSIONS: u32 = 32;

@group(0) @binding(1) var<uniform> params: array<GenerationParams, N_CHUNKS>;
@group(0) @binding(2) var<uniform> chunk_count: u32;
@group(0) @binding(3) var<storage, read_write> values: array<TerrainData>;

@compute @workgroup_size(1024)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let idx = invocation_id.x;

    let coords = expand(idx, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS);
    if coords.w >= chunk_count {
        // Most times we won't be generating the full number of chunks we can at the same time, 
        // so exit early if we aren't generating this one.
        return;
    }

    let param = params[coords.w];

    values[idx] = generate(param, coords);
}
