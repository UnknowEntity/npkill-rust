use std::{fmt, io::stdout, path::PathBuf, time::Duration};

use anyhow::Result;
use crossterm::event::{self, KeyCode};
use log::{error, info};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame, Terminal,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    command::HandlerCommand,
    event::HandlerEvent,
    time_helpers::{get_current_time, get_duration_human_time},
    ui::{
        guideline::guideline, status_block::status_block, title::title,
        version_block::version_block,
    },
};

#[derive(PartialEq, Eq)]
pub enum DirStatus {
    Loading,
    Ready,
    Deleting,
    Deleted,
    Error,
}

impl fmt::Display for DirStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DirStatus::Loading => write!(f, "LOADING"),
            DirStatus::Ready => write!(f, "READY"),
            DirStatus::Deleting => write!(f, "DELETING"),
            DirStatus::Deleted => write!(f, "DELETED"),
            DirStatus::Error => write!(f, "ERROR"),
        }
    }
}

struct NodeModulePath {
    bytes: Option<u64>,
    path: PathBuf,
    status: DirStatus,
}

impl NodeModulePath {
    fn new(path: PathBuf) -> NodeModulePath {
        NodeModulePath {
            bytes: None,
            path,
            status: DirStatus::Loading,
        }
    }

    fn update_size(&mut self, byte: u64) {
        self.bytes = Some(byte);
        self.status = DirStatus::Ready;
    }

    fn deleting(&mut self) {
        self.status = DirStatus::Deleting;
    }

    fn deleted(&mut self) -> u64 {
        self.status = DirStatus::Deleted;
        match self.bytes {
            Some(value) => value,
            None => 0,
        }
    }

    fn error(&mut self) {
        self.status = DirStatus::Error;
    }

    fn get_size_string(&self) -> String {
        match self.bytes {
            Some(bytes) => size::Size::from_bytes(bytes).to_string(),
            None => "__".to_owned(),
        }
    }

    fn get_size(&self) -> u64 {
        match self.bytes {
            Some(bytes) => bytes,
            None => 0,
        }
    }

    fn get_path(&self) -> String {
        return self.path.display().to_string();
    }
}

pub struct Data {
    items: Vec<NodeModulePath>,
    state: TableState,
    data_free: u64,
    data_contain: u64,
    start_timestamp: u64,
    end_timestamp: Option<u64>,
}

impl Data {
    fn new() -> Data {
        Data {
            items: vec![],
            state: TableState::default(),
            data_free: 0,
            data_contain: 0,
            start_timestamp: get_current_time(),
            end_timestamp: None,
        }
    }

    fn add_path(&mut self, path: PathBuf) -> usize {
        let last_index = self.items.len();
        self.items.push(NodeModulePath::new(path));
        last_index
    }

    fn update_size(&mut self, index: usize, byte: u64) {
        if let Some(value) = self.items.get_mut(index) {
            value.update_size(byte);
            self.data_contain += byte;
        }
    }

    fn get_free_space(&self) -> String {
        size::Size::from_bytes(self.data_free).to_string()
    }

    fn get_available_space(&self) -> String {
        size::Size::from_bytes(self.data_contain).to_string()
    }

    fn finish_search(&mut self) {
        self.end_timestamp = Some(get_current_time());
        info!(
            "Finish Search: {:?}",
            self.end_timestamp.unwrap_or_default()
        );
    }

    fn get_search_duration(&self) -> String {
        match self.end_timestamp {
            None => "__".to_owned(),
            Some(value) => get_duration_human_time(self.start_timestamp, value),
        }
    }

    fn delete_dir_path(&mut self, index: usize) -> Option<&PathBuf> {
        let Some(data) = self.items.get_mut(index) else {
            return None;
        };

        if data.status != DirStatus::Ready {
            return None;
        }

        data.deleting();

        return Some(&data.path);
    }

    fn update_delete_file(&mut self, index: usize, is_deleted: bool) {
        let Some(item) = self.items.get_mut(index) else {
            return;
        };

        if !is_deleted {
            item.error();
            return;
        }

        item.deleted();

        self.data_free += item.get_size();
    }

    fn get_selected(&self) -> Option<usize> {
        self.state.selected()
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub async fn start_ui(
    target_path: String,
    tx: &Sender<HandlerCommand>,
    rx: &mut Receiver<HandlerEvent>,
) -> Result<()> {
    tx.send(HandlerCommand::FindNodeModules(PathBuf::from(target_path)))
        .await?;

    let mut data = Data::new();
    // Configure Crossterm backend for ratatui
    let stdout = stdout();

    crossterm::terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    terminal.hide_cursor()?;

    loop {
        // Render
        terminal.draw(|rect| draw(rect, &mut data))?;

        if let Ok(handler_event) = rx.try_recv() {
            match handler_event {
                HandlerEvent::FindDir(path) => {
                    let index = data.add_path(path.clone());
                    if let Err(err) = tx.send(HandlerCommand::GetDirSize(index, path)).await {
                        error!("{err}");
                    };
                }
                HandlerEvent::FinishedFinding => data.finish_search(),
                HandlerEvent::FileSize(index, result) => match result {
                    Ok(size) => data.update_size(index, size),
                    Err(err) => error!("{err}"),
                },
                HandlerEvent::FileDeleted(index, result) => match result {
                    Ok(_) => data.update_delete_file(index, true),
                    Err(err) => error!("{err}"),
                },
            }
        }

        let crossterm_event = match crossterm::event::poll(Duration::from_millis(1000)) {
            Ok(result) => result,
            Err(err) => {
                error!("{err}");
                false
            }
        };

        if crossterm_event {
            let Ok(event::Event::Key(key)) = event::read() else {
                continue;
            };

            match key.code {
                KeyCode::Char('q') => {
                    if let Err(err) = tx.send(HandlerCommand::Quit).await {
                        error!("{err}");
                    };
                    break;
                }
                KeyCode::Up => data.previous(),
                KeyCode::Down => data.next(),
                KeyCode::Char(' ') => {
                    if let Some(index) = data.get_selected() {
                        if let Some(path) = data.delete_dir_path(index) {
                            if let Err(err) = tx
                                .send(HandlerCommand::DeleteDir(index, path.clone()))
                                .await
                            {
                                error!("{err}");
                            }
                        }
                    }
                }
                _ => {}
            };
        }
    }

    // Restore the terminal and close application
    terminal.clear()?;

    terminal.show_cursor()?;

    crossterm::terminal::disable_raw_mode()?;

    println!("");
    println!("\tFree space: {}", data.get_free_space());

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

    let status_block = status_block(
        data.get_available_space(),
        data.get_search_duration(),
        data.get_free_space(),
    );
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
                Cell::from(item.get_path().clone()),
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
