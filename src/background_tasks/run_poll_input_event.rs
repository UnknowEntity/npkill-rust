use log::error;
use std::{sync::mpsc::Sender, time::Duration};

use crossterm::event::{self, KeyCode};

use crate::InputEvent;

fn map_input_to_event(input_code: &KeyCode) -> Option<InputEvent> {
    match input_code {
        KeyCode::Char('q') => Some(InputEvent::Quit),
        KeyCode::Up => Some(InputEvent::Up),
        KeyCode::Down => Some(InputEvent::Down),
        KeyCode::Char(' ') => Some(InputEvent::Select),
        _ => None,
    }
}

pub fn run_poll_input_event(tx: &Sender<InputEvent>) {
    let event_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            match event_tx.send(InputEvent::Tick) {
                Err(_) => break,
                _ => {}
            }

            let crossterm_event = match crossterm::event::poll(Duration::from_millis(1000)) {
                Ok(result) => result,
                Err(err) => {
                    error!("{err}");
                    break;
                }
            };

            if crossterm_event {
                let Ok(event::Event::Key(key)) = event::read() else {
                    continue;
                };

                let Some(input_event) = map_input_to_event(&key.code) else {
                    continue;
                };

                if let Err(err) = event_tx.send(input_event) {
                    error!("{err}");
                    break;
                }
            }
        }
    });
}
