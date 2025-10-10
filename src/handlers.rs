use std::path::PathBuf;

use anyhow::Result;
use log::{error, info};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    command::HandlerCommand,
    event::HandlerEvent,
    file_tasks::{calculate_dir_size, delete_dir, find_node_modules},
};

pub async fn command_handlers(
    command_rx: &mut Receiver<HandlerCommand>,
    event_tx: &Sender<HandlerEvent>,
) -> Result<()> {
    loop {
        let Some(command) = command_rx.recv().await else {
            continue;
        };

        match command {
            HandlerCommand::FindNodeModules(path) => {
                info!("Initial Path: {:?}", path);
                let event_tx = event_tx.clone();
                tokio::spawn(async move {
                    let (tx, mut rx) = channel::<PathBuf>(100);

                    tokio::spawn(async move {
                        if let Err(err) = find_node_modules(path, &tx).await {
                            error!("{err}");
                        }
                        info!("Finished searching - Drop tx");
                    });

                    while let Some(dir) = rx.recv().await {
                        info!("node_modules found: {:?}", dir);
                        if let Err(err) = event_tx.send(HandlerEvent::FindDir(dir)).await {
                            error!("{err}");
                        }
                    }

                    if let Err(err) = event_tx.send(HandlerEvent::FinishedFinding).await {
                        error!("{err}");
                    }
                });
            }
            HandlerCommand::GetDirSize(index, path) => {
                let event_tx = event_tx.clone();
                tokio::spawn(async move {
                    let result = calculate_dir_size(path).await;

                    if let Err(err) = event_tx.send(HandlerEvent::FileSize(index, result)).await {
                        error!("{err}");
                    };
                });
            }
            HandlerCommand::DeleteDir(index, path) => {
                let event_tx = event_tx.clone();
                tokio::spawn(async move {
                    let result = delete_dir(path).await;

                    if let Err(err) = event_tx
                        .send(HandlerEvent::FileDeleted(index, result))
                        .await
                    {
                        error!("{err}");
                    };
                });
            }
            HandlerCommand::Quit => return Ok(()),
        }
    }
}
