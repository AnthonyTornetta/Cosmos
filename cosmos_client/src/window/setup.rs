use bevy::{
    prelude::{App, ResMut},
    window::Windows,
};

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_title("Cosmos".into());
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);
}

pub fn register(app: &mut App) {
    app.add_startup_system(setup_window);
}
