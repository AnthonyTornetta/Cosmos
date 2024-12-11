//! Handles various settings for the client read from the settings file.

use std::fs;

use bevy::{
    app::Update,
    log::error,
    prelude::{
        in_state, not, resource_changed, resource_exists, resource_exists_and_changed, AmbientLight, App, Commands, IntoSystemConfigs,
        IntoSystemSetConfigs, OnEnter, OnExit, Projection, Query, Res, ResMut, Resource, SystemSet, With,
    },
    utils::HashMap,
};
use cosmos_core::{
    registry::{create_registry, identifiable::Identifiable, Registry},
    state::GameState,
};
use serde::{Deserialize, Serialize};

use crate::{lang::Lang, rendering::MainCamera};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, PartialOrd, Ord)]
/// Category this setting belongs to (for display purposes only)
pub enum SettingCategory {
    /// Graphical related stuff
    Graphics,
    /// Mouse
    Mouse,
    /// Audio
    Audio,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
/// The data this setting contains (also encodes type information)
pub enum SettingData {
    /// Contains a float
    I32(i32),
    /// Contains a string
    String(String),
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
/// The data this setting contains (also encodes type information)
pub enum SettingConstraint {
    /// Setting contstraint for I32 values
    I32 {
        /// The minimum value (inclusive)
        min: i32,
        /// The maximum value (inclusive)
        max: i32,
    },
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Resource)]
/// A piece of data that can be set by the user
///
/// BEWARE: This is NOT guarenteed to be within any sort of bounds, since the user is free to change
/// the settings file to whatever they want.
pub struct Setting {
    id: u16,
    unlocalized_name: String,
    /// The data this stores. Please see warning for [`Self`].
    pub data: SettingData,
    /// The category this setting should be under (for display purposes)
    pub setting_category: SettingCategory,
    /// The setting's constraint
    pub constraint: Option<SettingConstraint>,
}

impl Setting {
    /// Creates a new setting that can be changed by the user
    pub fn new(
        unlocalized_name: impl Into<String>,
        default_value: SettingData,
        category: SettingCategory,
        constraint: Option<SettingConstraint>,
    ) -> Self {
        Self {
            data: default_value,
            id: 0,
            setting_category: category,
            unlocalized_name: unlocalized_name.into(),
            constraint,
        }
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct SettingBound<T: PartialOrd + Clone + std::fmt::Debug + PartialEq + Send + Sync + 'static> {
//     id: u16,
//     unlocalized_name: String,
//     min: Option<T>,
//     max: Option<T>,
// }

// impl<T: PartialOrd + Clone + std::fmt::Debug + PartialEq + Send + Sync + 'static> Identifiable for SettingBound<T> {
//     fn id(&self) -> u16 {
//         self.id
//     }

//     fn set_numeric_id(&mut self, id: u16) {
//         self.id = id;
//     }

//     fn unlocalized_name(&self) -> &str {
//         &self.unlocalized_name
//     }
// }

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

/// Ease-of-use methods for the `Registry<Setting>``
pub trait SettingsRegistry {
    /// If this setting contains an i32 value, it will return that. Otherwise, the default will be returned.
    fn i32_or(&self, unlocalized_name: &str, default: i32) -> i32;
    /// If this setting contains a &str value, it will return that. Otherwise, the default will be returned.
    fn str_or<'a>(&'a self, unlocalized_name: &str, default: &'a str) -> &'a str;
}

#[derive(Resource)]
/// Controlls how sensitive the camera should be to mouse movements
pub struct MouseSensitivity(pub f32);

impl SettingsRegistry for Registry<Setting> {
    fn i32_or(&self, unlocalized_name: &str, default: i32) -> i32 {
        self.from_id(unlocalized_name)
            .map(|x| if let SettingData::I32(d) = x.data { d } else { default })
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
    ambient_light.brightness = settings.i32_or("cosmos:brightness", 100) as f32;
}

fn load_mouse_sensitivity(mut commands: Commands, settings: Res<Registry<Setting>>) {
    commands.insert_resource(MouseSensitivity(settings.i32_or("cosmos:sensitivity", 75) as f32 / 100.0));
}

#[derive(Resource)]
/// The FOV desired by the client. This is not guarenteed to be within any bounds.
pub struct DesiredFov(pub f32);

fn load_fov(mut commands: Commands, settings: Res<Registry<Setting>>) {
    commands.insert_resource(DesiredFov(settings.i32_or("cosmos:fov", 90) as f32));
}

fn on_changed_desired_fov(mut q_cam: Query<&mut Projection, With<MainCamera>>, desired_fov: Res<DesiredFov>) {
    for mut proj in q_cam.iter_mut() {
        match proj.as_mut() {
            Projection::Perspective(persp) => {
                persp.fov = (desired_fov.0 / 180.0) * std::f32::consts::PI;
            }
            _ => error!("Unsupported main camera type -- not Projection::Perspective. Cannot change FOV!"),
        }
    }
}

fn register_settings(mut registry: ResMut<Registry<Setting>>) {
    registry.register(Setting::new(
        "cosmos:brightness",
        SettingData::I32(100),
        SettingCategory::Graphics,
        Some(SettingConstraint::I32 { min: 0, max: 200 }),
    ));

    registry.register(Setting::new(
        "cosmos:sensitivity",
        SettingData::I32(75),
        SettingCategory::Mouse,
        Some(SettingConstraint::I32 { min: 10, max: 200 }),
    ));

    registry.register(Setting::new(
        "cosmos:fov",
        SettingData::I32(90),
        SettingCategory::Graphics,
        Some(SettingConstraint::I32 { min: 30, max: 120 }),
    ));

    registry.register(Setting::new(
        "cosmos:music_volume",
        SettingData::I32(100),
        SettingCategory::Audio,
        Some(SettingConstraint::I32 { min: 0, max: 100 }),
    ));
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
                SettingData::I32(val) => format!("{val}"),
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
        SettingData::I32(_) => {
            let i32_parsed = value.parse::<i32>().ok()?;
            setting.data = SettingData::I32(i32_parsed);
        }
        SettingData::String(_) => {
            setting.data = SettingData::String(value.to_owned());
        }
    }

    Some(())
}

fn insert_settings_lang(mut langs: ResMut<Lang<Setting>>, settings: Res<Registry<Setting>>) {
    for setting in settings.iter() {
        langs.register(setting);
    }
}

fn init_settings_lang(mut commands: Commands) {
    commands.insert_resource(Lang::<Setting>::new("en_us", vec!["settings"]));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Set for changing + loading settings
pub enum SettingsSet {
    /// Settings changes should be here
    ChangeSettings,
    /// Responding to changed settings/initially loading settings should be done here.
    LoadSettings,
}

pub(super) fn register(app: &mut App) {
    create_registry::<Setting>(app, "cosmos:settings");

    app.add_systems(OnEnter(GameState::PreLoading), init_settings_lang)
        .add_systems(OnExit(GameState::PostLoading), insert_settings_lang);

    app.configure_sets(
        Update,
        (
            SettingsSet::ChangeSettings,
            SettingsSet::LoadSettings.run_if(resource_exists_and_changed::<Registry<Setting>>),
        )
            .chain(),
    );

    app.add_systems(
        Update,
        (
            on_change_loaded_settings.run_if(resource_exists::<SettingsSerialized>),
            serialize_settings.run_if(resource_changed::<Registry<Setting>>),
        )
            .chain()
            .after(SettingsSet::LoadSettings)
            .run_if(not(in_state(GameState::PreLoading))),
    );

    app.add_systems(OnEnter(GameState::PreLoading), register_settings);

    app.add_systems(OnEnter(GameState::Loading), load_settings).add_systems(
        Update,
        (
            (load_gamma, load_mouse_sensitivity, load_fov).in_set(SettingsSet::LoadSettings),
            on_changed_desired_fov,
        )
            .chain(),
    );
}
