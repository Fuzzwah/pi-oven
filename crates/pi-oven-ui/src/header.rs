// Conversation header strip. Reads from `ConversationHeader` placeholder
// fields; real values will arrive over the wire transport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::ConversationHeader;

pub fn render_header(area: Rect, buf: &mut Buffer, header: &ConversationHeader) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let title = truncate_to_width(&header.title, area.width as usize);
    let title_line = Line::from(Span::styled(
        title,
        Style::default().add_modifier(Modifier::BOLD),
    ));

    let stats = format!(
        "{}s · ↓{} · ↑{}",
        header.elapsed_secs,
        header.tokens_in,
        format_tokens(header.tokens_out),
    );
    let stats = truncate_to_width(&stats, area.width as usize);
    let stats_line = Line::from(Span::styled(
        stats,
        Style::default().add_modifier(Modifier::DIM),
    ));

    // Row 1: blank spacer; Row 2: title; Row 3: stats.
    let title_y = area.y.saturating_add(1);
    let stats_y = area.y.saturating_add(2);

    if title_y < area.y + area.height {
        let row = Rect { x: area.x, y: title_y, width: area.width, height: 1 };
        Paragraph::new(title_line).render(row, buf);
    }
    if stats_y < area.y + area.height {
        let row = Rect { x: area.x, y: stats_y, width: area.width, height: 1 };
        Paragraph::new(stats_line).render(row, buf);
    }
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        let v = (n as f64) / 1_000_000.0;
        format!("{:.1}M", v)
    } else if n >= 1_000 {
        let v = (n as f64) / 1_000.0;
        if v >= 100.0 {
            format!("{:.0}k", v)
        } else {
            format!("{:.1}k", v)
        }
    } else {
        n.to_string()
    }
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
