use ratatui::{
    layout::Alignment, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Paragraph, Wrap}
};

use crate::Data;

fn info<'a>(field_name: String, value: String) -> Line<'a> {
    Line::from(vec![
        Span::raw(field_name),
        Span::raw(": "),
        Span::styled(
            value,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

pub fn status_block<'a>(data: &Data) -> Paragraph<'a> {
    let total_size_value = data.get_available_space();

    let duration_value = data.get_search_duration();

    let free_space_value = data.get_free_space();

    let info_block = vec![
        info("Total size".to_owned(), total_size_value),
        info("Time".to_owned(), duration_value),
        info("Free space".to_owned(), free_space_value),
    ];
    Paragraph::new(info_block)
        .style(Style::default().bg(Color::Black))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
}
