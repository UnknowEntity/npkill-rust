use std::{sync::mpsc::Sender, path::Path, fs::DirEntry};

fn send_path(path: &Path, tx: &Sender<String>) {
    if let Err(err) = tx.send(path.display().to_string()) {
        println!("{err}");
    }
    return;
}

pub fn find_node_modules(path: &Path, tx: &Sender<String>) {
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

pub fn get_dir_size(path: &str) -> u64 {
    match fs_extra::dir::get_size(path) {
        Ok(size) => size,
        Err(_) => 0,
    }
}