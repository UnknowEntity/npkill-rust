use tui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::Paragraph,
};

use crate::ui::constant::TITLE;

pub fn title<'a>() -> Paragraph<'a> {
    Paragraph::new(TITLE)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .alignment(Alignment::Center)
}
