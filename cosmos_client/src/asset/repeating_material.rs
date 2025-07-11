//! A garbage repeated material. Don't use this.

// I think there's a bug in something causing a bunch of unused `check` function errors in this
// file. Probably one of the derives on the `UnlitRepeatedMaterial` struct.
#![allow(dead_code)]

use bevy::{
    asset::Asset,
    color::LinearRgba,
    prelude::{AlphaMode, App, Handle, Image, Material, MaterialPlugin},
    reflect::{Reflect, TypePath},
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};

#[repr(C, align(16))] // All WebGPU uniforms must be aligned to 16 bytes
#[derive(Clone, Copy, ShaderType, Debug, Hash, Eq, PartialEq, Default, Reflect)]
pub(crate) struct Repeats {
    pub(crate) horizontal: u32,
    pub(crate) vertical: u32,
    pub(crate) _wasm_padding1: u32,
    pub(crate) _wasm_padding2: u32,
}

#[derive(AsBindGroup, Debug, Clone, TypePath, Asset)]
pub(crate) struct UnlitRepeatedMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub(crate) texture: Handle<Image>,
    #[uniform(2)]
    pub(crate) repeats: Repeats,
    #[uniform(3)]
    pub(crate) color: LinearRgba,
}

impl Material for UnlitRepeatedMaterial {
    fn fragment_shader() -> ShaderRef {
        "cosmos/shaders/repeated.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Mask(0.5)
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<UnlitRepeatedMaterial>::default());
}
