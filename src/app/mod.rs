use std::{io::{stdout, Result}, sync::{mpsc::Receiver, Mutex, Arc}};

use tui::{backend::{CrosstermBackend, Backend}, Terminal, Frame, layout::{Layout, Direction, Constraint, Rect}, widgets::{Borders, Table, Row, Cell, Block}, style::{Style, Color, Modifier}};

use crate::{NodeModulePath, ui::{title, version_block, guideline, status_block}, DirStatus, InputEvent};

use super::Data;

pub fn start_ui(data: Arc<Mutex<Data>>, rx: &Receiver<InputEvent>) -> Result<()> {
    // Configure Crossterm backend for tui
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
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
        // TODO handle inputs here
        if let Ok(input_event) = rx.recv() {
            let mut data_event = match data.lock() {
                Ok(data) => data,
                Err(err) => {
                    println!("{err}");
                    break;
                }
            };
            match input_event {
                InputEvent::Quit => break,
                InputEvent::Tick => {},
                InputEvent::Up => data_event.previous(),
                InputEvent::Down => data_event.next(),
                InputEvent::Select => {},
            }
            drop(data_event);
        }
    }

    // Restore the terminal and close application
    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;

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


pub fn draw<B>(rect: &mut Frame<B>, data: &mut Data)
where
    B: Backend,
{
    let size = rect.size();

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

    // match &app.data {
    //     Some(data) => {
    //         let table = table(data);
    //         rect.render_stateful_widget(table, mid_chunk[1], &mut app.state);
    //     },
    //     None => {
    //         let placeholder = table_placeholder();
    //         rect.render_widget(placeholder, mid_chunk[1]);
    //     }
    // }

}

const ROW_BOTTOM_MARGIN: u16 = 1u16;

fn get_status_cell<'a>(status: &DirStatus) -> Cell<'a> {
    match status {
        DirStatus::Ready => Cell::from(status.to_string()).style(Style::default().fg(Color::Green)),
        DirStatus::Deleting => Cell::from(status.to_string()).style(Style::default().fg(Color::Yellow)),
        DirStatus::Deleted => Cell::from(status.to_string()).style(Style::default().fg(Color::Green).bg(Color::White)),
        DirStatus::Error => Cell::from(status.to_string()).style(Style::default().fg(Color::Red)),
        DirStatus::Loading => Cell::from(status.to_string()).style(Style::default().fg(Color::LightBlue))
    }
}

fn table<'a>(items: &Vec<NodeModulePath>) -> Table<'a> {
    let rows: Vec<Row> = items.iter().map(|item| {
        let cells = vec![
            Cell::from(item.path.clone()),
            Cell::from(item.get_size()),
            get_status_cell(&item.status)
        ];
        Row::new(cells).bottom_margin(ROW_BOTTOM_MARGIN)
    }).collect();

    Table::new(rows)
        .header(Row::new(vec!["Path", "Size", "Status"])
            .style(Style::default().fg(Color::Cyan))
            .bottom_margin(ROW_BOTTOM_MARGIN)
        )
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .widths(&[
            Constraint::Percentage(70),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ])
}