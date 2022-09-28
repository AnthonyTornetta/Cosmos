use bevy::{ecs::schedule::StateData, prelude::*, utils::HashSet};

/// Using the LoadingManager struct avoids passing ugly generics around the code, rather than directly using the LoadingStatus struct
#[derive(Default)]
pub struct LoadingManager {
    next_id: usize,
}

impl LoadingManager {
    pub fn register_loader(&mut self, event_writer: &mut EventWriter<AddLoadingEvent>) -> usize {
        self.next_id += 1;

        event_writer.send(AddLoadingEvent {
            loading_id: self.next_id,
        });

        self.next_id
    }

    pub fn finish_loading(&mut self, id: usize, event_writer: &mut EventWriter<DoneLoadingEvent>) {
        event_writer.send(DoneLoadingEvent { loading_id: id });
    }
}

struct LoadingStatus<T: StateData + Clone> {
    loaders: HashSet<usize>,
    done: bool, // at least one thing has to be processed before this is true. Prevents loading state from being advanced before stuff has a chance to get registered

    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_state: T,
}

impl<T: StateData + Clone> LoadingStatus<T> {
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

fn monitor_loading<T: StateData + Clone>(
    mut event_done_reader: EventReader<DoneLoadingEvent>,
    mut event_start_reader: EventReader<AddLoadingEvent>,
    mut loading_status: ResMut<LoadingStatus<T>>,
    mut state: ResMut<State<T>>,
) {
    for ev in event_start_reader.iter() {
        loading_status.loaders.insert(ev.loading_id);
    }

    for ev in event_done_reader.iter() {
        loading_status.done_loading(ev.loading_id.clone());
    }

    if loading_status.done {
        let cur_state = state.current().clone();

        if cur_state == loading_status.pre_loading_state {
            println!("Transitioning to loading state!");
            state.set(loading_status.loading_state.clone()).unwrap();
        } else if cur_state == loading_status.loading_state {
            println!("Transitioning to post loading state!");
            state
                .set(loading_status.post_loading_state.clone())
                .unwrap();
        } else if cur_state == loading_status.post_loading_state {
            println!("Transitioning to done state!");
            state.set(loading_status.done_state.clone()).unwrap();
        } else {
            panic!("Missing state!");
        }
    }
}

pub struct DoneLoadingEvent {
    pub loading_id: usize,
}

pub struct AddLoadingEvent {
    loading_id: usize,
}

impl<T: StateData + Clone> LoadingStatus<T> {
    fn done_loading(&mut self, id: usize) {
        self.loaders.remove(&id);

        if self.loaders.len() == 0 {
            self.done = true;
        }
    }
}

pub fn register<T: StateData + Clone>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
    post_loading_state: T,
    done_state: T,
) {
    app.add_event::<DoneLoadingEvent>()
        .add_event::<AddLoadingEvent>()
        // States cannot be changed during on_enter, and this prevents that from happening
        .add_system_set(SystemSet::on_update(pre_loading_state.clone()).with_system(monitor_loading::<T>))
        .add_system_set(SystemSet::on_update(loading_state.clone()).with_system(monitor_loading::<T>))
        .add_system_set(SystemSet::on_update(post_loading_state.clone()).with_system(monitor_loading::<T>))
        .insert_resource(LoadingStatus::new(
            pre_loading_state,
            loading_state,
            post_loading_state,
            done_state,
        ))
        .insert_resource(LoadingManager::default());
}
