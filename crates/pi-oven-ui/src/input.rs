use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub fn render_input(area: Rect, buf: &mut Buffer, text: &str, cursor_visible: bool) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height > 0 && inner.width > 0 {
        let line = if cursor_visible {
            let cursor = Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED));
            Line::from(vec![Span::raw(format!("> {text}")), cursor])
        } else {
            Line::from(Span::raw(format!("> {text}")))
        };
        Paragraph::new(line).render(inner, buf);
    }
}
