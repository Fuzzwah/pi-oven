// Bottom status bar. Reads from `StatusBar` placeholder fields; real values
// will arrive over the wire transport. Format:
//   `<model> - <ctx> - PR# <pr> - <branch>`  (all bold; PR segment is optional)

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::StatusBar;

pub fn render_status_bar(area: Rect, buf: &mut Buffer, status: &StatusBar) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let sep = " - ";
    let mut segments: Vec<String> = vec![status.model.clone(), status.ctx.clone()];
    if let Some(pr) = status.pr.as_ref() {
        segments.push(format!("PR# {}", pr));
    }
    segments.push(status.branch.clone());

    let raw = segments.join(sep);
    let truncated = truncate_to_width(&raw, area.width as usize);
    let line = Line::from(Span::styled(
        truncated,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    Paragraph::new(line).render(area, buf);
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
