use std::path::PathBuf;

use anyhow::Result;
use log::{error, info};
use tokio::sync::{
    broadcast,
    mpsc::{channel, Receiver, Sender},
};

use crate::{
    command::HandlerCommand,
    event::HandlerEvent,
    file_tasks::{calculate_dir_size, delete_dir, find_node_modules},
};

pub async fn command_handlers(
    shutdown_tx: &broadcast::Sender<bool>,
    command_rx: &mut Receiver<HandlerCommand>,
    event_tx: &Sender<HandlerEvent>,
) -> Result<()> {
    loop {
        let Some(command) = command_rx.recv().await else {
            continue;
        };

        match command {
            HandlerCommand::FindNodeModules(path) => {
                let event_tx = event_tx.clone();
                let mut shutdown_rx = shutdown_tx.subscribe();
                tokio::spawn(async move {
                    let (tx, mut rx) = channel::<PathBuf>(100);

                    tokio::spawn(async move {
                        if let Err(err) = find_node_modules(&mut shutdown_rx, path, &tx).await {
                            error!("{err}");
                        }
                    });

                    while let Some(dir) = rx.recv().await {
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
                let mut shutdown_rx = shutdown_tx.subscribe();
                tokio::spawn(async move {
                    let result = calculate_dir_size(&mut shutdown_rx, path).await;

                    if let Err(err) = event_tx.send(HandlerEvent::FileSize(index, result)).await {
                        error!("{err}");
                    };
                });
            }
            HandlerCommand::DeleteDir(index, path) => {
                let event_tx = event_tx.clone();
                let mut shutdown_rx = shutdown_tx.subscribe();
                tokio::spawn(async move {
                    let result = delete_dir(&mut shutdown_rx, path).await;

                    if let Err(err) = event_tx
                        .send(HandlerEvent::FileDeleted(index, result))
                        .await
                    {
                        error!("{err}");
                    };
                });
            }
            HandlerCommand::Quit => {
                if let Err(err) = shutdown_tx.send(true) {
                    error!("{err}");
                };
                info!("Gracefully shutdown backend");
                break;
            }
        }
    }

    return Ok(());
}
