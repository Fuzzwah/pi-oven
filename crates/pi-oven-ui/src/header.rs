// Conversation header strip. One row, centered + bold title from the
// `ConversationHeader` placeholder; real values will arrive over the wire
// transport.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::ConversationHeader;

pub fn render_header(area: Rect, buf: &mut Buffer, header: &ConversationHeader) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let title = truncate_to_width(&header.title, area.width as usize);
    let line = Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    Paragraph::new(line).alignment(Alignment::Center).render(area, buf);
}

fn truncate_to_width(s: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let total: usize = s.chars().count();
    if total <= max_cols {
        return s.to_string();
    }
    if max_cols == 1 {
        return "…".to_string();
    }
    let take = max_cols - 1;
    let mut out: String = s.chars().take(take).collect();
    out.push('…');
    out
}
