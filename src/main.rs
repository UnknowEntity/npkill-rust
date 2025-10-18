mod app;
mod command;
mod event;
mod file_tasks;
mod handlers;
mod time_helpers;
mod ui;

use crossbeam::channel::unbounded;
use log::error;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use time_helpers::{get_current_time, get_duration_human_time};

use crate::app::start_ui;
use crate::command::HandlerCommand;
use crate::event::HandlerEvent;
use crate::handlers::command_handlers;

fn main() {
    let args: Vec<String> = env::args().collect();

    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let start_ms = get_current_time();

    let (command_tx, mut command_rx) = unbounded::<HandlerCommand>();
    let (event_tx, mut event_rx) = unbounded::<HandlerEvent>();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_receiver = shutdown.clone();

    rayon::spawn(move || {
        if let Err(err) = command_handlers(shutdown_receiver, &mut command_rx, &event_tx) {
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

    let result = start_ui(target_path.clone(), &command_tx, &mut event_rx);

    if let Err(err) = result {
        error!("{err}");
    }

    let end_ms = get_current_time();

    println!("\tTime Run: {}", get_duration_human_time(start_ms, end_ms))
}
