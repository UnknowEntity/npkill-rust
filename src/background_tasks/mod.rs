use std::{sync::{Arc, Mutex, mpsc::{Sender, Receiver, self}}, time::Duration, env};

use crossterm::event::{self, KeyCode};

use crate::{Data, file_helpers::{get_dir_size, find_node_modules}, InputEvent};

fn map_input_to_event(input_code: &KeyCode) -> Option<InputEvent> {
    match input_code {
        KeyCode::Char('q') => Some(InputEvent::Quit),
        KeyCode::Up => Some(InputEvent::Up),
        KeyCode::Down => Some(InputEvent::Down),
        KeyCode::Char(' ') => Some(InputEvent::Select),
        _ => None
    }
}

fn get_all_path() -> Option<Receiver<String>> {
    let current_path = env::current_dir();
    // let current_path: Result<&std::path::Path, ()> = Ok(Path::new("D:\\personal-projects\\nodejs"));
    let (tx_path, rx_path): (Sender<String>, Receiver<String>) = mpsc::channel();
    match current_path {
        Err(err) => {
            println!("{err}");
            return None;
        },
        Ok(path) => {
            let tx = tx_path.clone();
            let path = path.to_owned();
            rayon::spawn(move || {
                find_node_modules(&path, &tx);
            });
        }
    };

    drop(tx_path);

    Some(rx_path)
}

fn get_and_calculate_dir_size(data: &Arc<Mutex<Data>>) {
    if let Some(rx_path) = get_all_path() {
        let share_data = data.clone();
        rayon::spawn(move || {
            let temp_data = share_data.clone();
            rayon::scope(move |t| {
                for receiver in rx_path.into_iter() {
                    if let Ok(mut data_lock) = temp_data.lock() {
                        let index = data_lock.add_path(receiver.clone());
                        let data_clone = temp_data.clone();
                        drop(data_lock);
                        t.spawn(move |_| {
                            let bytes = get_dir_size(&receiver);
                            if let Ok(mut data_lock) = data_clone.lock() {
                                data_lock.update_size(index, bytes);
                            }
                        });
                    }
                }
            });
    
            if let Ok(mut data_lock) = share_data.lock() {
                data_lock.finish_search();
                drop(data_lock);
            }
        });
    }
}

fn run_poll_input_event(tx: &Sender<InputEvent>) {
    let event_tx = tx.clone();
    rayon::spawn(move || {
        loop {
            let crossterm_event = match crossterm::event::poll(Duration::from_millis(1000)) {
                Ok(result) => result,
                Err(err) => {
                    println!("{err}");
                    break;
                },
            };
            if crossterm_event {
                if let event::Event::Key(key) = event::read().unwrap() {
                    if let Some(input_event) = map_input_to_event(&key.code) {
                        if let Err(err) = event_tx.send(input_event) {
                            println!("{err}");
                            break;
                        }
                    }
                }
            }

            match event_tx.send(InputEvent::Tick) {
                Err(_) => break,
                _ => {}
            }
        }
    });
}

pub fn run_background_task(data: &Arc<Mutex<Data>>, tx: &Sender<InputEvent>) {
    get_and_calculate_dir_size(data);
    run_poll_input_event(tx);
}