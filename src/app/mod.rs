use std::{io::{stdout, Result}, sync::{mpsc::{self, Sender, Receiver}, Mutex, Arc}, thread, time::Duration};

use crossterm::event::{self, KeyCode};
use tui::{backend::{CrosstermBackend, Backend}, Terminal, Frame, layout::{Layout, Direction, Constraint, Rect}, widgets::{Borders, Table, Row, Cell, Block}, style::{Style, Color, Modifier}};

use crate::NodeModulePath;

use super::Data;

enum InputEvent {
    Quit,
    Tick,
}

pub fn start_ui(data: Arc<Mutex<Data>>) -> Result<()> {
    // Configure Crossterm backend for tui
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let (tx, rx): (Sender<InputEvent>, Receiver<InputEvent>) = mpsc::channel();

    let event_tx = tx.clone();
        thread::spawn(move || {
            loop {
                // println!("loop");
                let crossterm_event = match crossterm::event::poll(Duration::from_millis(200)) {
                    Ok(result) => result,
                    Err(err) => {
                        println!("{err}");
                        break;
                    },
                };
                if crossterm_event {
                    if let event::Event::Key(key) = event::read().unwrap() {
                        if KeyCode::Char('q') == key.code {
                            if let Err(err) = event_tx.send(InputEvent::Quit) {
                                println!("{err}");
                            }
                            break;
                        }
                    }
                }
                if let Err(err) = event_tx.send(InputEvent::Tick) {
                    println!("{err}");
                    break;
                }
                
            }
        });

    drop(tx);
    loop {
        // println!("drawn");
        let data_lock = match data.lock() {
            Ok(data) => data,
            Err(err) => {
                println!("{err}");
                break;
            }
        };
        // Render
        terminal.draw(|rect| draw(rect, &data_lock))?;
        drop(data_lock);
        // TODO handle inputs here
        if let Ok(input_event) = rx.recv() {
            match input_event {
                InputEvent::Quit => break,
                InputEvent::Tick => continue,
            }
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


pub fn draw<B>(rect: &mut Frame<B>, data: &Data)
where
    B: Backend,
{
    let size = rect.size();

    check_size(&size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(7)].as_ref())
        .split(size);

    // let title = title();
    // rect.render_widget(title, chunks[0]);

    let version_chunk = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(2), Constraint::Min(2)].as_ref())
    .split(chunks[1]);

    // let version = version_block();
    // rect.render_widget(version, version_chunk[0]);

    let guideline_chunk = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
    .split(version_chunk[1]);

    // let guideline = guideline();
    // rect.render_widget(guideline, guideline_chunk[0]);

    let mid_chunk = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(3)].as_ref())
    .split(guideline_chunk[1]);

    // let status_block = status_block(app.total_size, app.time_init, app.free_space);
    // rect.render_widget(status_block, mid_chunk[0]);

    let table = table(&data.items);
    rect.render_widget(table, mid_chunk[1]);

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

// fn draw_title<'a>() -> Paragraph<'a> {
//     Paragraph::new("Plop with TUI")
//         .style(Style::default().fg(Color::LightCyan))
//         .alignment(Alignment::Center)
//         .block(
//             Block::default()
//                 .borders(Borders::ALL)
//                 .style(Style::default().fg(Color::White))
//                 .border_type(BorderType::Plain),
//         )
// }

const ROW_BOTTOM_MARGIN: u16 = 1u16;

fn table<'a>(items: &Vec<NodeModulePath>) -> Table<'a> {
    let rows: Vec<Row> = items.iter().map(|item| {
        let cells = vec![
            Cell::from(item.path.clone()),
            Cell::from(item.get_size()),
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