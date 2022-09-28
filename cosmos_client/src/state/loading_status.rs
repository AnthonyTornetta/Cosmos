use bevy::{prelude::*, utils::HashSet};

use super::game_state::GameState;

#[derive(Default)]
pub struct LoadingStatus {
    loaders: HashSet<usize>,
    next_id: usize,
    done: bool, // at least one thing has to be processed before this is true. Prevents loading state from being advanced before stuff has a chance to get registered
}

fn monitor_loading(
    mut event_reader: EventReader<DoneLoadingEvent>,
    mut loading_status: ResMut<LoadingStatus>,
    mut state: ResMut<State<GameState>>,
) {
    for ev in event_reader.iter() {
        loading_status.done_loading(ev.loading_id.clone());
    }

    if loading_status.done {
        match state.current() {
            GameState::PreLoading => {
                state.set(GameState::Loading).unwrap();
            }
            GameState::Loading => {
                state.set(GameState::PostLoading).unwrap();
            }
            GameState::PostLoading => {
                state.set(GameState::Connecting).unwrap();
            }
            _ => {}
        }
    }
}

pub struct DoneLoadingEvent {
    pub loading_id: usize,
}

impl LoadingStatus {
    pub fn register_loader(&mut self) -> usize {
        self.next_id += 1;
        self.loaders.insert(self.next_id);
        self.next_id
    }

    fn done_loading(&mut self, id: usize) {
        self.loaders.remove(&id);

        if self.loaders.len() == 0 {
            self.done = true;
        }
    }
}

pub fn register(app: &mut App) {
    app.add_event::<DoneLoadingEvent>()
        .add_system(monitor_loading)
        .insert_resource(LoadingStatus::default());
}
