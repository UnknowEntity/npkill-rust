mod app;
mod ui;
mod file_helpers;
mod time_helpers;
mod background_tasks;

use time_helpers::{get_current_time, get_duration_human_time};
use ratatui::widgets::TableState;
use std::{env, fmt, sync::{Arc, Mutex}};
use std::sync::mpsc::{Sender, self, Receiver};
use log::{error};

use crate::{app::start_ui, background_tasks::run_background_task};

use remove_dir_all::remove_dir_all;

#[derive(PartialEq, Eq)]
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

    fn get_size_string(&self) -> String {
        match self.bytes {
            Some(bytes) => size::Size::from_bytes(bytes).to_string(),
            None => "__".to_owned(),
        }
    }

    fn get_size(&self) -> u64 {
        match self.bytes {
            Some(bytes) => bytes,
            None => 0,
        }
    }
}

pub enum DeleteStatus {
    Deleted(usize),
    Error(usize),
}

pub struct Data {
    items: Vec<NodeModulePath>,
    state: TableState,
    data_free: u64,
    data_contain: u64,
    start_timestamp: u64,
    end_timestamp: Option<u64>,
    sender: Sender<DeleteStatus>,
}

impl Data {
    fn new(sender: &Sender<DeleteStatus>) -> Data {
        Data { 
            items: vec![], 
            state: TableState::default(), 
            data_free: 0, 
            data_contain: 0, 
            start_timestamp: get_current_time(), 
            end_timestamp: None,
            sender: sender.clone(),
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

    fn delete_dir(&mut self, index: usize) {
        let Some(data) = self.items.get_mut(index) else {
            return;
        };

        if data.status != DirStatus::Ready {
            return;
        }

        let path = data.path.clone();
        let sender = self.sender.clone();

        rayon::spawn(move || {
            let target_index = index.clone();
            let sender_result = match remove_dir_all(path) {
                Ok(_) => sender.send(DeleteStatus::Deleted(target_index)),
                Err(_) => sender.send(DeleteStatus::Error(target_index)),
            };

            if let Err(err) = sender_result {
                error!("{err}");
            }
        });

        data.deleting();
    }

    fn update_delete_file(&mut self, index: usize, is_deleted: bool) {
        let Some(item) = self.items.get_mut(index) else {
            return;
        };

        if !is_deleted {
            item.error();
            return;
        }

        item.deleted();
        
        self.data_free += item.get_size();
    }

    fn get_selected(&self) -> Option<usize> {
        self.state.selected()
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
    let args: Vec<String> = env::args().collect();
    
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    
    let start_ms = get_current_time();

    let (sender, receiver): (Sender<DeleteStatus>, Receiver<DeleteStatus>) = mpsc::channel();

    let data = Arc::new(Mutex::new(Data::new(&sender)));

    let (tx, rx): (Sender<InputEvent>, Receiver<InputEvent>) = mpsc::channel();

    drop(sender);

    run_background_task(&data, &tx, receiver, args.last().cloned());

    drop(tx);

    let result = start_ui(data.clone(), &rx);

    if let Err(err) = result {
        error!("{err}");
    }
    let end_ms = get_current_time();

    println!("     Time Run: {}", get_duration_human_time(start_ms, end_ms))
}
