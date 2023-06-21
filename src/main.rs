mod app;
mod ui;

use fs_extra;
use rayon;
use tui::widgets::TableState;
use std::{time::Duration, sync::{Arc, Mutex}, fmt};
use humantime::format_duration;
use std::{env, path::Path, sync::mpsc::{Sender, self, Receiver}, fs::DirEntry, time::{SystemTime, UNIX_EPOCH}};

use crate::app::start_ui;

// fn get_folder_name(path: &Path, tx: &Sender<String>) {
//     let tx = tx.clone();
//     let path = path.to_owned();
//     thread::spawn(move || {
//         find_node_modules(&path, &tx);
//     });
// }

fn find_node_modules(path: &Path, tx: &Sender<String>) {
    if path.is_dir() {
        if path.ends_with("node_modules") {
            send_path(path, tx);
            return;
        }


        let entries: Vec<DirEntry> = match path.read_dir() {
            Err(err) => {
                println!("{err}");
                return;
            },
            Ok(read_dir) => read_dir.flat_map(|entry| entry).collect(),
        };

        let dir = entries.iter().find(|entry| entry.path().is_dir() && entry.path().ends_with("node_modules"));

        if let Some(node_module) = dir {
            send_path(&node_module.path(), tx);
            return;
        }
        
        let is_skip = entries.iter().any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return false;
            }

            match path.file_name() {
                Some(name) => name == "package-lock.json" || name == "package.json",
                None => false,
            }
        });


        if is_skip {
            return;
        }


        for entry in entries.into_iter() {
            find_node_modules(&entry.path(), tx)
        }
    }
}

fn send_path(path: &Path, tx: &Sender<String>) {
    if let Err(err) = tx.send(path.display().to_string()) {
        println!("{err}");
    }
    return;
}

fn get_dir_size(path: &str) -> u64 {
    match fs_extra::dir::get_size(path) {
        Ok(size) => size,
        Err(_) => 0,
    }
}

fn get_current_time() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs() * 1000 +
            since_the_epoch.subsec_nanos() as u64 / 1_000_000
    
}

fn get_duration_human_time(start_ms: u64, end_ms: u64) -> String {
    format_duration(Duration::from_millis(end_ms - start_ms)).to_string()
}

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
        Data { items: vec![], state: TableState::default(), data_free: 0, data_contain: 0, start_timestamp: get_current_time(), end_timestamp: None }
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

fn main() {
    let start_ms = get_current_time();
    let current_path = env::current_dir();
    // let current_path: Result<&std::path::Path, ()> = Ok(Path::new("D:\\personal-projects\\nodejs"));
    let data = Arc::new(Mutex::new(Data::new()));
    let share_data = data.clone();
    rayon::scope(move |s| {
        let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        match current_path {
            Err(err) => {
                println!("{err}");
                return;
            },
            Ok(path) => {
                let tx = tx.clone();
                let path = path.to_owned();
                s.spawn(move |_| {
                    find_node_modules(&path, &tx);
                });
            }
        };
        drop(tx);
        s.spawn(move |_| {
            rayon::scope(move |t| {
                for receiver in rx.into_iter() {
                    if let Ok(mut data_lock) = share_data.lock() {
                        let index = data_lock.add_path(receiver.clone());
                        let data_clone = share_data.clone();
                        drop(data_lock);
                        t.spawn(move |_| {
                            let bytes = get_dir_size(&receiver);
                            if let Ok(mut data_lock) = data_clone.lock() {
                                data_lock.update_size(index, bytes);
                            }
                        });
                    }
                }

                if let Ok(mut data_lock) = share_data.lock() {
                    data_lock.finish_search();
                    drop(data_lock);
                }
            })
        });
        s.spawn( move |_| {
            let result = start_ui(data.clone());

            if let Err(err) = result {
                println!("{err}");
            }

        })
    });
    let end_ms = get_current_time();

    println!("Time Run: {}", get_duration_human_time(start_ms, end_ms))
}
