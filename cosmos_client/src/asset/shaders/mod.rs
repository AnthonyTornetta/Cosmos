// use bevy::prelude::{App, Handle, Image, Material, MaterialPlugin};
// use bevy::reflect::TypeUuid;
// use bevy::render::render_resource::{AsBindGroup, ShaderRef};

// #[derive(AsBindGroup, Debug, Clone, TypeUuid)]
// #[uuid = "9c5a0ddf-1eaf-41b4-9832-ed736fd26af3"]
// pub struct ArrayTextureMaterial {
//     #[texture(0, dimension = "2d_array")]
//     #[sampler(1)]
//     pub array_texture: Handle<Image>,
// }

// impl Material for ArrayTextureMaterial {
//     fn fragment_shader() -> ShaderRef {
//         "shaders/array_texture.wgsl".into()
//     }
// }

// pub fn register(app: &mut App) {
//     app.add_plugin(MaterialPlugin::<ArrayTextureMaterial>::default());
// }
