mod run_poll_input_event;
mod get_and_calculate_dir_size;
mod notify_delete;

use std::{
    sync::{
        mpsc::{ Receiver, Sender},
        Arc, Mutex,
    },
};

use run_poll_input_event::run_poll_input_event;
use get_and_calculate_dir_size::get_and_calculate_dir_size;
use notify_delete::notify_delete;

use crate::Data;
use crate::InputEvent;
use crate::DeleteStatus;


pub fn run_background_task(
    data: &Arc<Mutex<Data>>,
    tx: &Sender<InputEvent>,
    receiver: Receiver<DeleteStatus>,
) {
    get_and_calculate_dir_size(data);
    run_poll_input_event(tx);
    notify_delete(data, receiver)
}
