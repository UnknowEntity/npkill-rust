use std::{sync::mpsc::Sender, path::Path, fs::DirEntry};
use log::{error, info};

fn send_path(path: &Path, tx: &Sender<String>) {
    if let Err(err) = tx.send(path.display().to_string()) {
        error!("{err}");
    }
    return;
}

pub fn find_node_modules(path: &Path, tx: &Sender<String>) {
    if !path.is_dir() {
        return;
    }


    if path.ends_with("node_modules") {
        send_path(path, tx);
        return;
    }


    let entries: Vec<DirEntry> = match path.read_dir() {
        Ok(read_dir) => read_dir.flat_map(|entry| entry).collect(),
        Err(err) => {
            error!("{err}");
            return;
        },
    };

    let dir = entries.iter().find(|entry| entry.path().is_dir() && entry.path().ends_with("node_modules"));

    if let Some(node_module) = dir {
        send_path(&node_module.path(), tx);
        return;
    }

    for entry in entries.into_iter() {
        info!("{:?}", entry.file_name());
        find_node_modules(&entry.path(), tx)
    }
}

pub fn get_dir_size(path: &str) -> u64 {
    match fs_extra::dir::get_size(path) {
        Ok(size) => size,
        Err(_) => 0,
    }
}