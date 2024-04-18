use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity},
    reflect::Reflect,
};

#[derive(Component, Reflect)]
pub struct Shield {
    pub radius: f32,
    pub strength: f32,
    pub max_strength: f32,
    pub emitting_entity: Option<Entity>,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Shield>();
}
