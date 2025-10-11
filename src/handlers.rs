use std::{collections::VecDeque, path::PathBuf};

use anyhow::Result;
use log::{error, info};
use tokio::sync::{
    broadcast,
    mpsc::{self, channel, error::TryRecvError, Receiver, Sender},
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
    let mut command_queue: VecDeque<HandlerCommand> = VecDeque::new();

    let mut command_executing: u8 = 0;
    let max_command_execute: u8 = 10;

    let (task_tx, mut task_rx) = mpsc::channel::<()>(100);

    loop {
        tokio::task::yield_now().await;
        let command = match command_rx.try_recv() {
            Ok(command) => match command {
                HandlerCommand::Quit => {
                    info!("Quit command receive");
                    handler(shutdown_tx, command, event_tx, &task_tx).await;
                    break;
                }
                command @ _ => Some(command),
            },
            Err(err) => match err {
                TryRecvError::Disconnected => break,
                TryRecvError::Empty => None,
            },
        };

        if let Some(upcoming_event) = command {
            command_queue.push_back(upcoming_event);
        }

        while !task_rx.is_empty() {
            match task_rx.try_recv() {
                Ok(_) => command_executing -= 1,
                Err(err) => {
                    error!("{err}");
                    break;
                }
            }
        }

        if command_executing == max_command_execute {
            continue;
        }

        let Some(next_command) = command_queue.pop_front() else {
            continue;
        };

        handler(shutdown_tx, next_command, event_tx, &task_tx).await;
        command_executing += 1;
    }

    return Ok(());
}

async fn handler(
    shutdown_tx: &broadcast::Sender<bool>,
    next_command: HandlerCommand,
    event_tx: &Sender<HandlerEvent>,
    task_tx: &Sender<()>,
) {
    let clone_event_tx = event_tx.clone();
    let clone_task_tx = task_tx.clone();
    let mut clone_shutdown_rx = shutdown_tx.subscribe();

    match next_command {
        HandlerCommand::FindNodeModules(path) => {
            tokio::spawn(async move {
                let (tx, mut rx) = channel::<PathBuf>(100);

                tokio::spawn(async move {
                    if let Err(err) = find_node_modules(&mut clone_shutdown_rx, path, &tx).await {
                        error!("{err}");
                    }
                });

                while let Some(dir) = rx.recv().await {
                    if let Err(err) = clone_event_tx.send(HandlerEvent::FindDir(dir)).await {
                        error!("{err}");
                    }
                }

                if let Err(err) = clone_event_tx.send(HandlerEvent::FinishedFinding).await {
                    error!("{err}");
                }

                send_task_finished(&clone_task_tx).await;
            });
        }
        HandlerCommand::GetDirSize(index, path) => {
            tokio::spawn(async move {
                let result = calculate_dir_size(&mut clone_shutdown_rx, path).await;

                if let Err(err) = clone_event_tx
                    .send(HandlerEvent::FileSize(index, result))
                    .await
                {
                    error!("{err}");
                };

                send_task_finished(&clone_task_tx).await;
            });
        }
        HandlerCommand::DeleteDir(index, path) => {
            tokio::spawn(async move {
                let result = delete_dir(&mut clone_shutdown_rx, path).await;

                if let Err(err) = clone_event_tx
                    .send(HandlerEvent::FileDeleted(index, result))
                    .await
                {
                    error!("{err}");
                };

                send_task_finished(&clone_task_tx).await;
            });
        }
        HandlerCommand::Quit => {
            if let Err(err) = shutdown_tx.send(true) {
                error!("{err}");
            };
            info!("Gracefully shutdown backend");
        }
    }
}

async fn send_task_finished(task_tx: &Sender<()>) {
    if let Err(err) = task_tx.send(()).await {
        error!("{err}");
    };
}
