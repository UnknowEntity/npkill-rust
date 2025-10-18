use std::ffi::OsStr;
use std::fs::read_dir;
use std::fs::remove_dir;
use std::fs::remove_file;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;
use crossbeam::channel::Sender;
use log::error;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use walkdir::WalkDir;

pub fn find_node_modules(path: PathBuf, tx: &Sender<PathBuf>) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    if path.file_name().unwrap_or(OsStr::new("")) == "node_modules" {
        match tx.send(path) {
            Ok(_) => {}
            Err(err) => error!("{err}"),
        };
        return Ok(());
    }

    let mut dirs: Vec<PathBuf> = vec![];

    for entry in read_dir(path.as_path())? {
        let dir_entry: DirEntry = entry?;

        let file_type = dir_entry.file_type()?;

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() && dir_entry.file_name().as_os_str() == "node_modules" {
            tx.send(dir_entry.path())?;
            return Ok(());
        }

        if file_type.is_dir() {
            dirs.push(dir_entry.path());
        }
    }

    for dir in dirs {
        let tx_clone = tx.clone();
        rayon::spawn(move || {
            if let Err(err) = find_node_modules(dir, &tx_clone) {
                error!("{err}");
            }
        });
    }

    Ok(())
}

pub fn compute_total_size(shutdown: &Arc<AtomicBool>, root: PathBuf) -> Result<u64> {
    let total: u64 = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .par_bridge() // stream the iterator into Rayon threads
        .filter_map(|entry_res| match entry_res {
            Ok(entry) => Some(entry),
            Err(err) => {
                error!("{err}");
                None
            } // you can log errors instead of dropping
        })
        .take_any_while(|_| !shutdown.load(Ordering::Relaxed))
        .map(|entry| {
            // get metadata (blocking) and return size or 0 on error
            match entry.metadata() {
                Ok(m) if m.is_file() => m.len(),
                _ => 0,
            }
        })
        .sum();

    Ok(total)
}

pub fn delete_dir(shutdown: &Arc<AtomicBool>, target_path: PathBuf) -> Result<()> {
    let read_dir = read_dir(target_path.as_path())?;

    rayon::scope(|s| {
        for entry in read_dir {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            let shutdown_clone = shutdown.clone();

            s.spawn(move |_| {
                let dir_entry: DirEntry = match entry {
                    Ok(entry) => entry,
                    Err(err) => {
                        error!("{err}");
                        return;
                    }
                };

                let file_type = match dir_entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(err) => {
                        error!("{err}");
                        return;
                    }
                };

                if file_type.is_symlink() {
                    match remove_file(dir_entry.path()) {
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                            // Try as directory symlink
                            if let Err(err) = remove_dir(dir_entry.path()) {
                                error!("{err}");
                            }
                        }
                        Err(err) => error!("{err}"),
                    };

                    return;
                }

                if file_type.is_file() {
                    if let Err(err) = remove_file(dir_entry.path()) {
                        error!("{err}");
                    }

                    return;
                }

                if file_type.is_dir() {
                    if let Err(err) = delete_dir(&shutdown_clone, dir_entry.path()) {
                        error!("{err}");
                    };
                }
            });
        }
    });

    remove_dir(target_path)?;

    return Ok(());
}
