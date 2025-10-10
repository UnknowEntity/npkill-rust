use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Result;
use log::{error, info};
use tokio::fs::{read_dir, remove_dir, remove_file, symlink_metadata};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;

pub async fn find_node_modules(
    shutdown_rx: &mut Receiver<bool>,
    path: PathBuf,
    tx: &Sender<PathBuf>,
) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let mut paths: Vec<PathBuf> = vec![path];

    while let Some(entry) = paths.pop() {
        if shutdown_rx.try_recv().is_ok_and(|signal| signal) {
            info!("Gracefully shutdown find_node_modules");
            return Ok(());
        }

        if entry.file_name().unwrap_or(OsStr::new("")) == "node_modules" {
            match tx.send(entry).await {
                Ok(_) => {}
                Err(err) => error!("{err}"),
            };
            continue;
        }

        let mut read_dir = read_dir(entry.as_path()).await?;

        let mut temp_dir: Vec<PathBuf> = vec![];

        while let Some(dir_entry) = read_dir.next_entry().await? {
            if dir_entry.path().is_dir() && dir_entry.file_name().as_os_str() == "node_modules" {
                tx.send(dir_entry.path()).await?;
                temp_dir = vec![];
                continue;
            }

            match symlink_metadata(dir_entry.path()).await {
                Ok(symlink_metadata) => {
                    if symlink_metadata.is_symlink() {
                        continue;
                    }
                }
                Err(err) => {
                    error!("{err}");
                    continue;
                }
            };

            if dir_entry.path().is_dir() {
                temp_dir.push(dir_entry.path());
            }
        }

        if !temp_dir.is_empty() {
            paths.append(&mut temp_dir);
        }
    }

    Ok(())
}

pub async fn calculate_dir_size(
    shutdown_rx: &mut Receiver<bool>,
    target_path: PathBuf,
) -> Result<u64> {
    let mut total_size: u64 = 0;

    if target_path.is_file() {
        return Ok(target_path.metadata()?.len());
    }

    let mut paths: Vec<PathBuf> = vec![target_path];

    while let Some(entry) = paths.pop() {
        if shutdown_rx.try_recv().is_ok_and(|signal| signal) {
            info!("Gracefully shutdown calculate_dir_size");
            return Ok(0);
        }

        let mut read_dir = read_dir(entry.as_path()).await?;

        while let Some(dir_entry) = read_dir.next_entry().await? {
            if dir_entry.path().is_file() {
                let file_size = match dir_entry.path().metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(err) => {
                        error!("{err}");
                        0
                    }
                };

                total_size += file_size;
            }

            match symlink_metadata(dir_entry.path()).await {
                Ok(symlink_metadata) => {
                    if symlink_metadata.is_symlink() {
                        continue;
                    }
                }
                Err(err) => {
                    error!("{err}");
                    continue;
                }
            };

            if dir_entry.path().is_dir() {
                paths.push(dir_entry.path());
            }
        }
    }

    return Ok(total_size);
}

pub async fn delete_dir(shutdown_rx: &mut Receiver<bool>, target_path: PathBuf) -> Result<()> {
    if target_path.is_file() {
        remove_file(target_path).await?;
        return Ok(());
    }

    let mut paths: Vec<PathBuf> = vec![target_path];
    let mut clean_up_dirs: Vec<PathBuf> = vec![];

    while let Some(entry) = paths.pop() {
        if shutdown_rx.try_recv().is_ok_and(|signal| signal) {
            info!("Gracefully shutdown delete_dir");
            return Ok(());
        }

        let mut read_dir = read_dir(entry.as_path()).await?;
        while let Some(dir_entry) = read_dir.next_entry().await? {
            if dir_entry.path().is_file() {
                remove_file(dir_entry.path()).await?;
            }

            match symlink_metadata(dir_entry.path()).await {
                Ok(symlink_metadata) => {
                    if symlink_metadata.is_symlink() {
                        continue;
                    }
                }
                Err(err) => {
                    error!("{err}");
                    continue;
                }
            };

            if dir_entry.path().is_dir() {
                paths.push(dir_entry.path());
            }
        }
        clean_up_dirs.push(entry);
    }

    while let Some(dir) = clean_up_dirs.pop() {
        remove_dir(dir).await?;
    }

    return Ok(());
}
