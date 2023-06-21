use tui::{widgets::{Paragraph, Wrap}, layout::Alignment, style::{Style, Color, Modifier}, text::{Spans, Span}};

use crate::Data;

const TITLE: &'static str = r"
                       __                           __   .__.__  .__   
_______ __ __  _______/  |_            ____ ______ |  | _|__|  | |  |  
\_  __ \  |  \/  ___/\   __\  ______  /    \\____ \|  |/ /  |  | |  |  
|  | \/  |  /\___ \  |  |   /_____/ |   |  \  |_> >    <|  |  |_|  |__
|__|  |____//____  > |__|           |___|  /   __/|__|_ \__|____/____/
                 \/                      \/|__|        \/             
";

pub fn title<'a>() -> Paragraph<'a> {
    Paragraph::new(TITLE).style(Style::default().fg(Color::White).bg(Color::Black))
    .alignment(Alignment::Center)
}

pub fn version_block<'a>() -> Paragraph<'a> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    Paragraph::new(VERSION)
        .style(Style::default().fg(Color::LightBlue))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
}

const GUIDELINE: &'static str = r"Select with CURSORS
Delete with SPACE
Quit with 'q'";

pub fn guideline<'a>() -> Paragraph<'a> {
    Paragraph::new(GUIDELINE).style(Style::default().bg(Color::Yellow).fg(Color::Black))
}

fn info<'a>(field_name: String, value: String) -> Spans<'a> {
    Spans::from(vec![
        Span::raw(field_name),
        Span::raw(": "),
        Span::styled(value, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    ])
}

pub fn status_block<'a>(data: &Data) -> Paragraph<'a> {
    let total_size_value = data.get_available_space();

    let duration_value = data.get_search_duration();

    let free_space_value = data.get_free_space();

    let info_block = vec![
        info("Total size".to_owned(), total_size_value),
        info("Time".to_owned(), duration_value),
        info("Free space".to_owned(), free_space_value)
    ];
    Paragraph::new(info_block)
        .style(Style::default().bg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
}

// pub fn () {
    
// }