mod app;
mod command;
mod event;
mod file_tasks;
mod handlers;
mod time_helpers;
mod ui;

use log::error;
use std::env;
use time_helpers::{get_current_time, get_duration_human_time};
use tokio::sync::{broadcast, mpsc};

use crate::app::start_ui;
use crate::command::HandlerCommand;
use crate::event::HandlerEvent;
use crate::handlers::command_handlers;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let start_ms = get_current_time();

    let (command_tx, mut command_rx) = mpsc::channel::<HandlerCommand>(100);
    let (event_tx, mut event_rx) = mpsc::channel::<HandlerEvent>(100);
    let (shutdown_tx, _) = broadcast::channel::<bool>(100);

    tokio::spawn(async move {
        if let Err(err) = command_handlers(&shutdown_tx, &mut command_rx, &event_tx).await {
            error!("{err}");
        };
    });

    let cwd = match env::current_dir() {
        Ok(cwd) => cwd.as_path().display().to_string(),
        Err(err) => {
            error!("{err}");
            return;
        }
    };

    let target_path = args.get(1).unwrap_or(&cwd);

    let result = start_ui(target_path.clone(), &command_tx, &mut event_rx).await;

    if let Err(err) = result {
        error!("{err}");
    }

    let end_ms = get_current_time();

    println!("\tTime Run: {}", get_duration_human_time(start_ms, end_ms))
}
