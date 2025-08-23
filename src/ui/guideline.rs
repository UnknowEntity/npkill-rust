use ratatui::{
    style::{Color, Style},
    widgets::Paragraph,
};

use crate::ui::constant::GUIDELINE;

pub fn guideline<'a>() -> Paragraph<'a> {
    Paragraph::new(GUIDELINE).style(Style::default().bg(Color::Yellow).fg(Color::Black))
}
