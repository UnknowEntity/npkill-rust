use fs_extra;
use size::Size;
use rayon;
use std::time::Duration;
use humantime::format_duration;
use std::{env, thread, path::Path, sync::mpsc::{Sender, self, Receiver}, fs::DirEntry, time::{SystemTime, UNIX_EPOCH}};

fn get_folder_name(path: &Path, tx: &Sender<String>) {
    let tx = tx.clone();
    let path = path.to_owned();
    thread::spawn(move || {
        find_node_modules(&path, &tx);
    });
}

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

fn get_dir_size(path: &str) -> std::string::String {
    let bytes = match fs_extra::dir::get_size(path) {
        Ok(size) => size,
        Err(_) => 0,
    };

    return Size::from_bytes(bytes).to_string();
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

fn main() {
    let start_ms = get_current_time();
    let current_path = env::current_dir();
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    match current_path {
        Err(err) => {
            println!("{err}");
            return;
        },
        Ok(path) => {
            get_folder_name(&path, &tx);
        }
    };
    drop(tx);
    rayon::scope(move |s| {
        for receiver in rx.into_iter() {
            s.spawn(move |_| println!("{}: {}", receiver, get_dir_size(&receiver)));
        }

    });

    let end_ms = get_current_time();

    println!("Time Run: {}", get_duration_human_time(start_ms, end_ms))
}
