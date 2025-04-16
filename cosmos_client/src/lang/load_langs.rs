use bevy::prelude::{App, OnExit, Res, ResMut};
use cosmos_core::{
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use super::Lang;

pub(super) fn insert_langs<T: Identifiable>(mut t_lang: ResMut<Lang<T>>, t_reg: Res<Registry<T>>) {
    for t in t_reg.iter() {
        t_lang.register(t);
    }
}

pub(super) fn register<T: Identifiable>(app: &mut App, read_from: Vec<&'static str>) {
    app.insert_resource(Lang::<T>::new("en_us", read_from));

    app.add_systems(OnExit(GameState::LoadingData), insert_langs::<T>);
}
