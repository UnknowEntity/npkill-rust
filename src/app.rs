use std::{
    io::{stdout, Result},
    sync::{mpsc::Receiver, Arc, Mutex},
};

use log::error;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame, Terminal,
};

use crate::{
    ui::{
        guideline::guideline, status_block::status_block, title::title,
        version_block::version_block,
    },
    DirStatus, InputEvent, NodeModulePath,
};

use super::Data;

pub fn start_ui(data: Arc<Mutex<Data>>, rx: &Receiver<InputEvent>) -> Result<()> {
    // Configure Crossterm backend for ratatui
    let stdout = stdout();

    if let Err(err) = crossterm::terminal::enable_raw_mode() {
        error!("{err}");

        return Err(err);
    }

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(err) => {
            error!("{err}");

            return Err(err);
        }
    };

    if let Err(err) = terminal.clear() {
        error!("{err}");

        return Err(err);
    }

    if let Err(err) = terminal.hide_cursor() {
        error!("{err}");

        return Err(err);
    }

    loop {
        let mut data_lock = match data.lock() {
            Ok(data) => data,
            Err(err) => {
                println!("{err}");
                break;
            }
        };
        // Render
        terminal.draw(|rect| draw(rect, &mut data_lock))?;

        drop(data_lock);

        let Ok(input_event) = rx.recv() else {
            continue;
        };

        let mut data_event = match data.lock() {
            Ok(data) => data,
            Err(err) => {
                println!("{err}");
                break;
            }
        };

        match input_event {
            InputEvent::Quit => break,
            InputEvent::Tick => {}
            InputEvent::Up => data_event.previous(),
            InputEvent::Down => data_event.next(),
            InputEvent::Select => {
                if let Some(index) = data_event.get_selected() {
                    data_event.delete_dir(index);
                }
            }
        }

        drop(data_event);
    }

    // Restore the terminal and close application
    if let Err(err) = terminal.clear() {
        error!("{err}");

        return Err(err);
    }

    if let Err(err) = terminal.show_cursor() {
        error!("{err}");

        return Err(err);
    }

    if let Err(err) = crossterm::terminal::disable_raw_mode() {
        error!("{err}");

        return Err(err);
    }

    Ok(())
}

fn check_size(rect: &Rect) {
    if rect.width < 52 {
        panic!("Require width >= 52, (got {})", rect.width);
    }
    if rect.height < 28 {
        panic!("Require height >= 28, (got {})", rect.height);
    }
}

pub fn draw(rect: &mut Frame, data: &mut Data) {
    let size = rect.area();

    check_size(&size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(7)].as_ref())
        .split(size);

    let title = title();
    rect.render_widget(title, chunks[0]);

    let version_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)].as_ref())
        .split(chunks[1]);

    let version = version_block();
    rect.render_widget(version, version_chunk[0]);

    let guideline_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
        .split(version_chunk[1]);

    let guideline = guideline();
    rect.render_widget(guideline, guideline_chunk[0]);

    let mid_chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
        .split(guideline_chunk[1]);

    let status_block = status_block(data);
    rect.render_widget(status_block, mid_chunk[0]);

    let table = table(&data.items);
    rect.render_stateful_widget(table, mid_chunk[1], &mut data.state);
}

const ROW_BOTTOM_MARGIN: u16 = 1u16;

fn get_status_cell<'a>(status: &DirStatus) -> Cell<'a> {
    match status {
        DirStatus::Ready => Cell::from(status.to_string()).style(Style::default().fg(Color::Green)),
        DirStatus::Deleting => {
            Cell::from(status.to_string()).style(Style::default().fg(Color::Yellow))
        }
        DirStatus::Deleted => {
            Cell::from(status.to_string()).style(Style::default().fg(Color::Green).bg(Color::White))
        }
        DirStatus::Error => Cell::from(status.to_string()).style(Style::default().fg(Color::Red)),
        DirStatus::Loading => {
            Cell::from(status.to_string()).style(Style::default().fg(Color::LightBlue))
        }
    }
}

fn table<'a>(items: &Vec<NodeModulePath>) -> Table<'a> {
    let rows: Vec<Row> = items
        .iter()
        .map(|item| {
            let cells = vec![
                Cell::from(item.path.clone()),
                Cell::from(item.get_size_string()),
                get_status_cell(&item.status),
            ];
            Row::new(cells).bottom_margin(ROW_BOTTOM_MARGIN)
        })
        .collect();

    Table::new(
        rows,
        &[
            Constraint::Percentage(70),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ],
    )
    .header(
        Row::new(vec!["Path", "Size", "Status"])
            .style(Style::default().fg(Color::Cyan))
            .bottom_margin(ROW_BOTTOM_MARGIN),
    )
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
}
