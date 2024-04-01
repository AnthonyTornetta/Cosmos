#import "cosmos/shaders/biosphere/default_generation.wgsl"::{
    default_generate,
};

#import "cosmos/shaders/biosphere/generation_utils.wgsl"::{
    GenerationParams,
    TerrainData,
}

fn generate(
    param: GenerationParams,
    coords: vec4u
) -> TerrainData {
    return default_generate(param, coords);
}
