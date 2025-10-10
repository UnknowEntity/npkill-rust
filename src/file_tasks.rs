use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Result;
use log::{error, info};
use tokio::fs::{read_dir, remove_dir, remove_file};
use tokio::sync::mpsc::Sender;

pub async fn find_node_modules(path: PathBuf, tx: &Sender<PathBuf>) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }

    let mut paths: Vec<PathBuf> = vec![path];

    while let Some(entry) = paths.pop() {
        if entry.file_name().unwrap_or(OsStr::new("")) == "node_modules" {
            match tx.send(entry).await {
                Ok(_) => {}
                Err(err) => error!("{err}"),
            };
            continue;
        }

        let mut read_dir = read_dir(entry.as_path()).await?;

        let mut temp_dir: Vec<PathBuf> = vec![];

        info!("Root: {:?}", entry);

        while let Some(dir_entry) = read_dir.next_entry().await? {
            if dir_entry.path().is_dir() && dir_entry.file_name().as_os_str() == "node_modules" {
                tx.send(dir_entry.path()).await?;
                temp_dir = vec![];
                info!("Branch: {:?}", dir_entry.path());
                continue;
            }

            if dir_entry.path().is_dir() {
                temp_dir.push(dir_entry.path());
            }
        }

        if !temp_dir.is_empty() {
            paths.append(&mut temp_dir);
        }
    }

    info!("Finish paths");

    Ok(())
}

pub async fn calculate_dir_size(target_path: PathBuf) -> Result<u64> {
    let mut total_size: u64 = 0;

    if target_path.is_file() {
        return Ok(target_path.metadata()?.len());
    }

    let mut paths: Vec<PathBuf> = vec![target_path];

    while let Some(entry) = paths.pop() {
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

            if dir_entry.path().is_dir() {
                paths.push(dir_entry.path());
            }
        }
    }

    return Ok(total_size);
}

pub async fn delete_dir(target_path: PathBuf) -> Result<()> {
    if target_path.is_file() {
        remove_file(target_path).await?;
        return Ok(());
    }

    let mut paths: Vec<PathBuf> = vec![target_path];
    let mut clean_up_dirs: Vec<PathBuf> = vec![];

    while let Some(entry) = paths.pop() {
        let mut read_dir = read_dir(entry.as_path()).await?;
        while let Some(dir_entry) = read_dir.next_entry().await? {
            if dir_entry.path().is_file() {
                remove_file(dir_entry.path()).await?;
            }

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
