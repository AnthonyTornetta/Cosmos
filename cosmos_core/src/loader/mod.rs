//! This is kinda stupid, but I don't have any better ideas right now.
//!
//! To make state transitions better, this is used to flag when states should be moved.
//! You should be good to just ignore this, and let it do its thing.
//!
//! Just if you ever remove a call to `register_loader` or `finish_loading` you may have to add it to another
//! system in that state.

use bevy::{prelude::*, utils::HashSet};

/// Using the LoadingManager struct avoids passing ugly generics around the code, rather than directly using the LoadingStatus struct
#[derive(Default, Resource)]
pub struct LoadingManager {
    next_id: usize,
}

impl LoadingManager {
    /// Registers that there is something loaded.
    ///
    /// Returns the ID that should be passed to `finish_loading` once this is done.
    pub fn register_loader(&mut self, event_writer: &mut EventWriter<AddLoadingEvent>) -> usize {
        self.next_id += 1;

        event_writer.send(AddLoadingEvent {
            loading_id: self.next_id,
        });

        self.next_id
    }

    /// Finishes loading for this id.
    pub fn finish_loading(&mut self, id: usize, event_writer: &mut EventWriter<DoneLoadingEvent>) {
        event_writer.send(DoneLoadingEvent { loading_id: id });
    }
}

#[derive(Resource)]
struct LoadingStatus<T: States + Clone + Copy> {
    loaders: HashSet<usize>,
    done: bool, // at least one thing has to be processed before this is true. Prevents loading state from being advanced before stuff has a chance to get registered

    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_state: T,
}

impl<T: States + Clone + Copy> LoadingStatus<T> {
    pub fn new(
        pre_loading_state: T,
        loading_state: T,
        post_loading_state: T,
        done_state: T,
    ) -> Self {
        Self {
            loaders: HashSet::new(),
            done: false,

            pre_loading_state,
            loading_state,
            post_loading_state,
            done_state,
        }
    }
}

fn monitor_loading<T: States + Clone + Copy>(
    mut event_done_reader: EventReader<DoneLoadingEvent>,
    mut event_start_reader: EventReader<AddLoadingEvent>,
    mut loading_status: ResMut<LoadingStatus<T>>,
    state: Res<State<T>>,
    mut state_changer: ResMut<NextState<T>>,
) {
    for ev in event_start_reader.iter() {
        loading_status.loaders.insert(ev.loading_id);
    }

    for ev in event_done_reader.iter() {
        loading_status.done_loading(ev.loading_id);
    }

    if loading_status.done {
        let cur_state = state.0;

        if cur_state == loading_status.pre_loading_state {
            println!("Transitioning to loading state!");
            state_changer.set(loading_status.loading_state);
        } else if cur_state == loading_status.loading_state {
            println!("Transitioning to post loading state!");
            state_changer.set(loading_status.post_loading_state);
        } else if cur_state == loading_status.post_loading_state {
            println!("Transitioning to done state!");
            state_changer.set(loading_status.done_state);
        } else {
            panic!("Missing state!");
        }

        loading_status.done = false;
    }
}

/// Sent when something is done loading during the game's loading states.
pub struct DoneLoadingEvent {
    /// The loading id assigned by `register_loader`
    pub loading_id: usize,
}

/// Sent when something starts loading during the game's loading states.
pub struct AddLoadingEvent {
    loading_id: usize,
}

impl<T: States + Clone + Copy> LoadingStatus<T> {
    fn done_loading(&mut self, id: usize) {
        self.loaders.remove(&id);

        if self.loaders.is_empty() {
            self.done = true;
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_state: T,
) {
    app.add_event::<DoneLoadingEvent>()
        .add_event::<AddLoadingEvent>()
        .add_system(
            monitor_loading::<T>.run_if(
                in_state(pre_loading_state)
                    .or_else(in_state(loading_state).or_else(in_state(post_loading_state))),
            ),
        )
        .insert_resource(LoadingStatus::new(
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_state,
        ))
        .insert_resource(LoadingManager::default());
}
