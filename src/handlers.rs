use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::{error, info};

use crate::{
    command::HandlerCommand,
    event::HandlerEvent,
    file_tasks::{compute_total_size, delete_dir, find_node_modules},
};

pub fn command_handlers(
    shutdown: Arc<AtomicBool>,
    command_rx: &mut Receiver<HandlerCommand>,
    event_tx: &Sender<HandlerEvent>,
) -> Result<()> {
    let mut command_queue: VecDeque<HandlerCommand> = VecDeque::new();
    loop {
        let command = match command_rx.try_recv() {
            Ok(command) => match command {
                HandlerCommand::Quit => {
                    info!("Quit command receive");
                    handler(&shutdown.clone(), command, event_tx);
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

        let Some(next_command) = command_queue.pop_front() else {
            continue;
        };

        handler(&shutdown.clone(), next_command, event_tx);
    }

    return Ok(());
}

fn handler(
    shutdown: &Arc<AtomicBool>,
    next_command: HandlerCommand,
    event_tx: &Sender<HandlerEvent>,
) {
    let clone_event_tx = event_tx.clone();
    let clone_shutdown = shutdown.clone();

    match next_command {
        HandlerCommand::FindNodeModules(path) => {
            rayon::spawn(move || {
                let (tx, rx) = unbounded::<PathBuf>();

                rayon::spawn(move || {
                    if let Err(err) = find_node_modules(path, &tx) {
                        error!("{err}");
                    }
                });

                while let Ok(dir) = rx.recv() {
                    if let Err(err) = clone_event_tx.send(HandlerEvent::FindDir(dir)) {
                        error!("{err}");
                    }
                }

                if let Err(err) = clone_event_tx.send(HandlerEvent::FinishedFinding) {
                    error!("{err}");
                }
            });
        }
        HandlerCommand::GetDirSize(index, path) => {
            rayon::spawn(move || {
                let result = compute_total_size(&clone_shutdown, path);

                if let Err(err) = clone_event_tx.send(HandlerEvent::FileSize(index, result)) {
                    error!("{err}");
                };
            });
        }
        HandlerCommand::DeleteDir(index, path) => {
            rayon::spawn(move || {
                let result = delete_dir(&clone_shutdown, path);

                if let Err(err) = clone_event_tx.send(HandlerEvent::FileDeleted(index, result)) {
                    error!("{err}");
                };
            });
        }
        HandlerCommand::Quit => {
            shutdown.swap(false, Ordering::Relaxed);
            info!("Gracefully shutdown backend");
        }
    }
}
