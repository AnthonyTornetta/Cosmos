//! The menu that first appears when you load into the game.

use bevy::{picking::pointer::PointerPress, post_process::bloom::Bloom, prelude::*, render::view::Hdr, window::Monitor};
use bevy_kira_audio::SpatialAudioReceiver;
use bevy_rapier3d::plugin::DefaultRapierContext;
use cosmos_core::state::GameState;

use crate::ui::UiSystemSet;

use super::components::show_cursor::ShowCursor;

mod disconnect_screen;
mod menu_panorama;
mod multiplayer_screen;
mod settings_screen;
mod singleplayer_screen;
mod title_screen;
mod triggers;

#[derive(Component)]
struct DespawnOnSwitchState;

#[derive(Component)]
struct MainMenuCamera;

#[derive(Component)]
struct MainMenuRootUiNode;

#[derive(Component)]
struct BackgroundColorNode;

#[derive(Debug, Default, Resource)]
struct MainMenuTime(f32);

#[derive(Component)]
/// This component prevents something from being despawned when a transition to the main menu happens.
pub struct SurviveMainMenu;

#[derive(Debug, Resource, Default, Clone, Copy, PartialEq, Eq)]
/// The "substate" of the menu we are in -- will be redone when bevy 0.14 is integrated.
pub enum MainMenuSubState {
    #[default]
    /// The landing screen that shows the title
    TitleScreen,
    /// Settings menu
    Settings,
    /// When the player is disconnected from a server, this will display the latest disconnect message.
    Disconnect,
    /// The singleplayer menu
    Singleplayer,
    /// The multiplayer menu
    Multiplayer,
}

fn despawn_all_main_menu_ents<T: Component>(mut commands: Commands, q_main_menu_entities: Query<Entity, With<T>>) {
    for e in q_main_menu_entities.iter() {
        commands.entity(e).despawn();
    }
}

fn create_main_menu_background_node(mut commands: Commands, q_main_menu_camera: Query<Entity, With<MainMenuCamera>>) {
    let Ok(cam_ent) = q_main_menu_camera.single() else {
        return;
    };

    commands.spawn((
        BackgroundColorNode,
        UiTargetCamera(cam_ent),
        DespawnOnSwitchState,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_content: AlignContent::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
    ));
}

fn create_main_menu_root_node(q_bg_node: Query<Entity, With<BackgroundColorNode>>, mut commands: Commands) {
    let Ok(ent) = q_bg_node.single() else {
        return;
    };

    commands.entity(ent).with_children(|p| {
        p.spawn((
            MainMenuRootUiNode,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_content: AlignContent::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ));
    });
}

fn spin_camera(mut q_main_menu_camera: Query<&mut Transform, With<MainMenuCamera>>, time: Res<Time>) {
    for mut trans in q_main_menu_camera.iter_mut() {
        trans.rotation *= Quat::from_axis_angle(Vec3::Y, time.delta_secs() / 30.0);
    }
}

fn fade_in_background(
    mut q_root_node: Query<&mut BackgroundColor, With<BackgroundColorNode>>,
    mut main_menu_time: ResMut<MainMenuTime>,
    time: Res<Time>,
) {
    for mut bg in q_root_node.iter_mut() {
        const MIN_A: f32 = 0.6;

        let alpha_now = (1.0 / (6.0 * main_menu_time.0) + MIN_A).min(1.0);
        let old_bg = Srgba::from(bg.0);

        bg.0 = Srgba {
            red: old_bg.red,
            green: old_bg.green,
            blue: old_bg.blue,
            alpha: alpha_now,
        }
        .into();
    }

    main_menu_time.0 += time.delta_secs();
}

fn create_main_menu_camera(mut commands: Commands) {
    commands.spawn((
        DespawnOnSwitchState,
        MainMenuCamera,
        Hdr,
        Camera { ..Default::default() },
        Transform::default(),
        Projection::from(PerspectiveProjection {
            fov: (90.0 / 180.0) * std::f32::consts::PI,
            ..default()
        }),
        Bloom { ..Default::default() },
        Name::new("Main Menu Camera"),
        Camera3d::default(),
        IsDefaultUiCamera,
        SpatialAudioReceiver,
        ShowCursor,
    ));
}

fn create_main_menu_resource(
    q_entity: Query<
        Entity,
        (
            // Maybe we make this opt-in in the future?
            Without<SurviveMainMenu>,
            Without<Window>,
            Without<DefaultRapierContext>,
            Without<ChildOf>,
            Without<Observer>,
            Without<Monitor>,
            Without<Pointer<PointerPress>>,
        ),
    >,
    mut commands: Commands,
    mm_resource: Option<Res<MainMenuSubState>>,
) {
    for ent in q_entity.iter() {
        commands.entity(ent).despawn();
    }

    // trigger change detection, even if the resource alrea*x
    commands.insert_resource(mm_resource.map(|x| x.clone()).unwrap_or_default());
    commands.init_resource::<MainMenuTime>();
}

fn remove_main_menu_resource(mut commands: Commands) {
    commands.remove_resource::<MainMenuSubState>();
    commands.remove_resource::<MainMenuTime>();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Contains the ordering of operations that should take place within the main menu
pub enum MainMenuSystemSet {
    /// If there is an old menu, cleans it up
    CleanupMenu,
    /// Creates your new menu
    InitializeMenu,
    /// Listens to any menu events and responds to them
    UpdateMenu,
}

/// Checks if we are in the main menu state (used for system run conditions)
pub fn in_main_menu_state(state: MainMenuSubState) -> impl Fn(Option<Res<MainMenuSubState>>) -> bool {
    move |mms: Option<Res<MainMenuSubState>>| mms.map(|x| *x == state).unwrap_or(false)
}

pub(super) fn register(app: &mut App) {
    menu_panorama::register(app);
    title_screen::register(app);
    disconnect_screen::register(app);
    triggers::register(app);
    settings_screen::register(app);
    multiplayer_screen::register(app);
    singleplayer_screen::register(app);

    app.configure_sets(
        Update,
        (
            MainMenuSystemSet::CleanupMenu,
            MainMenuSystemSet::InitializeMenu.before(UiSystemSet::DoUi),
            MainMenuSystemSet::UpdateMenu.after(UiSystemSet::FinishUi),
        )
            .chain()
            .run_if(in_state(GameState::MainMenu)),
    );

    app.add_systems(
        OnEnter(GameState::MainMenu),
        (create_main_menu_resource, create_main_menu_camera, create_main_menu_background_node).chain(),
    ); //create_main_menu);

    app.add_systems(Update, (spin_camera, fade_in_background).run_if(in_state(GameState::MainMenu)));

    app.add_systems(
        Update,
        (despawn_all_main_menu_ents::<MainMenuRootUiNode>, create_main_menu_root_node)
            .chain()
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::CleanupMenu),
    );
    app.add_systems(
        OnExit(GameState::MainMenu),
        (despawn_all_main_menu_ents::<DespawnOnSwitchState>, remove_main_menu_resource).chain(),
    );
}
