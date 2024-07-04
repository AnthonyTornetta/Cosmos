//! Handles various settings for the client read from the settings file.

use std::fs;

use bevy::{
    app::Update,
    prelude::{
        in_state, not, resource_changed, resource_exists, AmbientLight, App, Commands, IntoSystemConfigs, OnEnter, OnExit, Res, ResMut,
        Resource,
    },
    utils::HashMap,
};
use cosmos_core::registry::{create_registry, identifiable::Identifiable, Registry};
use serde::{Deserialize, Serialize};

use crate::state::game_state::GameState;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum SettingCategory {
    Graphics,
    Mouse,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum SettingData {
    F32(f32),
    String(String),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Resource)]
pub struct Setting {
    id: u16,
    unlocalized_name: String,
    data: SettingData,
    setting_category: SettingCategory,
}

impl Setting {
    pub fn new(unlocalized_name: impl Into<String>, default_value: SettingData, category: SettingCategory) -> Self {
        Self {
            data: default_value,
            id: 0,
            setting_category: category,
            unlocalized_name: unlocalized_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingBound<T: PartialOrd + Clone + std::fmt::Debug + PartialEq + Send + Sync + 'static> {
    id: u16,
    unlocalized_name: String,
    min: Option<T>,
    max: Option<T>,
}

impl<T: PartialOrd + Clone + std::fmt::Debug + PartialEq + Send + Sync + 'static> Identifiable for SettingBound<T> {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl Identifiable for Setting {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

pub trait SettingsRegistry {
    fn f32_or(&self, unlocalized_name: &str, default: f32) -> f32;
    fn str_or<'a>(&'a self, unlocalized_name: &str, default: &'a str) -> &'a str;
}

#[derive(Resource)]
pub struct MouseSensitivity(pub f32);

impl SettingsRegistry for Registry<Setting> {
    fn f32_or(&self, unlocalized_name: &str, default: f32) -> f32 {
        self.from_id(unlocalized_name)
            .map(|x| if let SettingData::F32(d) = x.data { d } else { default })
            .unwrap_or(default)
    }

    fn str_or<'a>(&'a self, unlocalized_name: &str, default: &'a str) -> &'a str {
        self.from_id(unlocalized_name)
            .map(|x| {
                if let SettingData::String(d) = &x.data {
                    d.as_str()
                } else {
                    default
                }
            })
            .unwrap_or(default)
    }
}

fn load_gamma(settings: Res<Registry<Setting>>, mut ambient_light: ResMut<AmbientLight>) {
    ambient_light.brightness = settings.f32_or("cosmos:brightness", 100.0);
}

fn load_mouse_sensitivity(mut commands: Commands, settings: Res<Registry<Setting>>) {
    commands.insert_resource(MouseSensitivity(settings.f32_or("cosmos:sensitivity", 0.75)));
}

fn register_settings(mut registry: ResMut<Registry<Setting>>) {
    registry.register(Setting::new(
        "cosmos:brightness",
        SettingData::F32(100.0),
        SettingCategory::Graphics,
    ));
    registry.register(Setting::new("cosmos:sensitivity", SettingData::F32(0.75), SettingCategory::Mouse));
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Resource, Default)]
struct SettingsSerialized(HashMap<String, String>);

fn load_settings(mut commands: Commands) {
    let settings_serialized =
        toml::from_str::<SettingsSerialized>(fs::read_to_string("settings/settings.toml").unwrap_or("".to_string()).as_str())
            .unwrap_or_default();

    commands.insert_resource(settings_serialized);
}

fn serialize_settings(settings: Res<Registry<Setting>>) {
    if settings.is_empty() {
        // They haven't been loaded yet, so don't overwrite existing settings
        return;
    }

    let mut serialized = SettingsSerialized::default();

    for setting in settings.iter() {
        serialized.0.insert(
            setting.unlocalized_name().to_owned(),
            match &setting.data {
                SettingData::F32(val) => format!("{val}"),
                SettingData::String(str) => str.to_owned(),
            },
        );
    }

    _ = fs::create_dir("settings");

    fs::write(
        "settings/settings.toml",
        toml::to_string(&serialized).expect("Error parsing settings into toml."),
    )
    .expect("Error saving settings file!");
}

fn on_change_loaded_settings(
    mut commands: Commands,
    settings_serialized: Res<SettingsSerialized>,
    mut settings: ResMut<Registry<Setting>>,
) {
    // Trigger change detection for the settings resource even if no change happens.
    //
    // This is to cause a serialization of the settings in the `serialize_settings` system to ensure
    // the settings file has everything.
    let settings = settings.as_mut();

    for (setting, value) in settings_serialized.0.iter() {
        process_setting(setting, value, settings);
    }

    commands.remove_resource::<SettingsSerialized>();
}

fn process_setting(setting: &str, value: &str, settings: &mut Registry<Setting>) -> Option<()> {
    let setting = settings.from_id_mut(setting)?;

    match setting.data {
        SettingData::F32(_) => {
            let f32_parsed = value.parse::<f32>().ok()?;
            setting.data = SettingData::F32(f32_parsed);
        }
        SettingData::String(_) => {
            setting.data = SettingData::String(value.to_owned());
        }
    }

    Some(())
}

pub(super) fn register(app: &mut App) {
    create_registry::<Setting>(app, "cosmos:settings");

    app.add_systems(
        Update,
        (
            on_change_loaded_settings.run_if(resource_exists::<SettingsSerialized>),
            serialize_settings.run_if(resource_changed::<Registry<Setting>>),
        )
            .chain()
            .run_if(not(in_state(GameState::PreLoading))),
    );

    app.add_systems(OnEnter(GameState::PreLoading), register_settings);

    app.add_systems(OnEnter(GameState::Loading), load_settings.chain())
        .add_systems(OnExit(GameState::Loading), (load_gamma, load_mouse_sensitivity));
}
