mod app;

use fs_extra;
use rayon;
use std::{time::Duration, sync::{Arc, Mutex}};
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

struct NodeModulePath {
    bytes: Option<u64>,
    path: String,
}

impl NodeModulePath {
    fn new(path: String) -> NodeModulePath {
        NodeModulePath { bytes: None, path }
    }

    fn update_size(&mut self, byte: u64) {
        self.bytes = Some(byte);
    }

    fn get_size(&self) -> String {
        match self.bytes {
            Some(bytes) => size::Size::from_bytes(bytes).to_string(),
            None => "__".to_owned(),
        }
    }
}

pub struct Data {
    items: Vec<NodeModulePath>
}

impl Data {
    fn new() -> Data {
        Data { items: vec![] }
    }

    fn add_path(&mut self, path: String) -> usize {
        let last_index = self.items.len();
        self.items.push(NodeModulePath::new(path));
        last_index
    }

    fn update_size(&mut self, index: usize, byte: u64) {
        if let Some(value) = self.items.get_mut(index) {
            value.update_size(byte);
        }
    }
}

fn main() {
    let start_ms = get_current_time();
    let current_path = env::current_dir();
    // let current_path: Result<&std::path::Path, ()> = Ok(Path::new("D:\\projects"));
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
            rayon::scope(move |s| {
                for receiver in rx.into_iter() {
                    if let Ok(mut data_lock) = share_data.lock() {
                        let index = data_lock.add_path(receiver.clone());
                        let data_clone = share_data.clone();
                        drop(data_lock);
                        s.spawn(move |_| {
                            let bytes = get_dir_size(&receiver);
                            if let Ok(mut data_lock) = data_clone.lock() {
                                data_lock.update_size(index, bytes);
                            }
                        });
                    }
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
