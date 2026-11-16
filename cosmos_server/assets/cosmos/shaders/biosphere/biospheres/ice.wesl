import package::cosmos::shaders::biosphere::default_generation::{
    default_generate,
};

import package::cosmos::shaders::biosphere::generation_utils::{
    GenerationParams,
    TerrainData,
};

fn generate(
    param: GenerationParams,
    coords: vec4u
) -> TerrainData {
    return default_generate(param, coords);
}
