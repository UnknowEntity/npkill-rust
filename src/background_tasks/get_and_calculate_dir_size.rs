use log::error;
use std::{
    env::{self}, fs, path::PathBuf, sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    }
};

use crate::{
    file_helpers::{find_node_modules, get_dir_size},
    Data,
};

fn get_all_path(target_path: Option<String>) -> Option<Receiver<String>> {
    let path = if let Some(input_path) = target_path {
        match fs::exists(&input_path) {
            Ok(true) => {},
            Ok(false) => {
                error!("Path not exist");
                return None;
            },
            Err(err) => {
                error!("{err}");
                return None;
            }
        };

        let metadata = match fs::metadata(&input_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                error!("{err}");
                return None;
            }
        };

        if !metadata.is_dir() {
            error!("Path is not a dir: {}", input_path);
            return None;
        };

        PathBuf::from(input_path)
    } else {
        match env::current_dir() {
            Err(err) => {
                error!("{err}");
                return None;
            }
            Ok(path) => path,
        }
    };

    let (tx_path, rx_path): (Sender<String>, Receiver<String>) = mpsc::channel();

    let tx = tx_path.clone();

    rayon::spawn(move || {
        find_node_modules(&path, &tx);
    });

    drop(tx_path);

    return Some(rx_path);
}

pub fn get_and_calculate_dir_size(data: &Arc<Mutex<Data>>, target_path: Option<String>) {
    let Some(rx_path) = get_all_path(target_path) else {
        return;
    };

    let share_data = data.clone();

    rayon::spawn(move || {
        let temp_data = share_data.clone();
        rayon::scope(move |t| {
            for receiver in rx_path.into_iter() {
                let Ok(mut data_lock) = temp_data.lock() else {
                    continue;
                };

                let index = data_lock.add_path(receiver.clone());
                let data_clone = temp_data.clone();

                drop(data_lock);

                t.spawn(move |_| {
                    let bytes = get_dir_size(&receiver);

                    if let Ok(mut data_lock) = data_clone.lock() {
                        data_lock.update_size(index, bytes);
                    }
                });
            }
        });

        let Ok(mut data_lock) = share_data.lock() else {
            return;
        };

        data_lock.finish_search();
        drop(data_lock);
    });
}
