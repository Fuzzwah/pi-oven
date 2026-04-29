// Bottom status bar. Reads from `StatusBar` placeholder fields; real values
// will arrive over the wire transport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::StatusBar;

pub fn render_status_bar(area: Rect, buf: &mut Buffer, status: &StatusBar) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let sep = " · ";
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(
        status.model.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(sep.to_string()));
    spans.push(Span::raw(format!("ctx:{}%", status.ctx_pct)));
    if let Some(pr) = status.pr {
        spans.push(Span::raw(sep.to_string()));
        spans.push(Span::styled(
            format!("PR #{}", pr),
            Style::default().fg(Color::Magenta),
        ));
    }
    spans.push(Span::raw(sep.to_string()));
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let max_cols = area.width as usize;
    let branch = if used >= max_cols {
        String::new()
    } else {
        truncate_to_width(&status.branch, max_cols - used)
    };
    spans.push(Span::styled(branch, Style::default().add_modifier(Modifier::DIM)));

    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn truncate_to_width(s: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let total = s.chars().count();
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
