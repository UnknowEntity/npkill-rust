mod app;
mod ui;
mod file_helpers;
mod time_helpers;
mod background_tasks;

use time_helpers::{get_current_time, get_duration_human_time};
use tui::widgets::TableState;
use std::{sync::{Arc, Mutex}, fmt};
use std::sync::mpsc::{Sender, self, Receiver};

use crate::{app::start_ui, background_tasks::run_background_task};

pub enum DirStatus {
    Loading,
    Ready,
    Deleting,
    Deleted,
    Error,
}

impl fmt::Display for DirStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DirStatus::Loading => write!(f, "LOADING"),
            DirStatus::Ready => write!(f, "READY"),
            DirStatus::Deleting => write!(f, "DELETING"),
            DirStatus::Deleted => write!(f, "DELETED"),
            DirStatus::Error => write!(f, "ERROR"),
        }
    }
}

struct NodeModulePath {
    bytes: Option<u64>,
    path: String,
    status: DirStatus,
}

impl NodeModulePath {
    fn new(path: String) -> NodeModulePath {
        NodeModulePath { bytes: None, path, status: DirStatus::Loading }
    }

    fn update_size(&mut self, byte: u64) {
        self.bytes = Some(byte);
        self.status = DirStatus::Ready;
    }

    fn deleting(&mut self) {
        self.status = DirStatus::Deleting;
    }

    fn deleted(&mut self) -> u64 {
        self.status = DirStatus::Deleted;
        match self.bytes {
            Some(value) => value,
            None => 0,
        }
    }

    fn error(&mut self) {
        self.status = DirStatus::Error;
    }

    fn get_size(&self) -> String {
        match self.bytes {
            Some(bytes) => size::Size::from_bytes(bytes).to_string(),
            None => "__".to_owned(),
        }
    }
}

pub struct Data {
    items: Vec<NodeModulePath>,
    state: TableState,
    data_free: u64,
    data_contain: u64,
    start_timestamp: u64,
    end_timestamp: Option<u64>,
}

impl Data {
    fn new() -> Data {
        Data { 
            items: vec![], 
            state: TableState::default(), 
            data_free: 0, 
            data_contain: 0, 
            start_timestamp: get_current_time(), 
            end_timestamp: None 
        }
    }

    fn add_path(&mut self, path: String) -> usize {
        let last_index = self.items.len();
        self.items.push(NodeModulePath::new(path));
        last_index
    }

    fn update_size(&mut self, index: usize, byte: u64) {
        if let Some(value) = self.items.get_mut(index) {
            value.update_size(byte);
            self.data_contain += byte;
        }
    }

    fn get_free_space(&self) -> String {
        size::Size::from_bytes(self.data_free).to_string()
    }

    fn get_available_space(&self) -> String {
        size::Size::from_bytes(self.data_contain).to_string()
    }

    fn finish_search(&mut self) {
        self.end_timestamp = Some(get_current_time());
    }

    fn get_search_duration(&self) -> String {
        match self.end_timestamp {
            None => "__".to_owned(),
            Some(value) => get_duration_human_time(self.start_timestamp, value)
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                } 
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub enum InputEvent {
    Quit,
    Up,
    Down,
    Select,
    Tick,
}

fn main() {
    let start_ms = get_current_time();
    let data = Arc::new(Mutex::new(Data::new()));
    let (tx, rx): (Sender<InputEvent>, Receiver<InputEvent>) = mpsc::channel();
    run_background_task(&data, &tx);
    drop(tx);
    let result = start_ui(data.clone(), &rx);

    if let Err(err) = result {
        println!("{err}");
    }
    let end_ms = get_current_time();

    println!("Time Run: {}", get_duration_human_time(start_ms, end_ms))
}
