// Hotkey legend row. Reads from `AppState.legend` placeholders; this slice
// does not actually route any of the listed hotkeys.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

pub fn render_legend(area: Rect, buf: &mut Buffer, entries: &[(String, String)]) {
    if area.height == 0 || area.width == 0 || entries.is_empty() {
        return;
    }

    let max_cols = area.width as usize;
    let separator = "  ";
    let sep_w = separator.chars().count();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    let mut truncated = false;

    for (i, (keys, action)) in entries.iter().enumerate() {
        let entry_chars = keys.chars().count() + 1 + action.chars().count();
        let needed = if i == 0 { entry_chars } else { entry_chars + sep_w };
        if used + needed > max_cols {
            truncated = true;
            break;
        }
        if i > 0 {
            spans.push(Span::raw(separator.to_string()));
            used += sep_w;
        }
        spans.push(Span::styled(
            keys.clone(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {}", action)));
        used += entry_chars;
    }

    if truncated {
        let ellipsis = "…";
        while used + 1 + sep_w > max_cols && !spans.is_empty() {
            if let Some(last) = spans.pop() {
                used -= last.content.chars().count();
            }
        }
        if used > 0 {
            spans.push(Span::raw(format!("{}{}", separator, ellipsis)));
        } else {
            spans.push(Span::raw(ellipsis.to_string()));
        }
    }

    Paragraph::new(Line::from(spans)).render(area, buf);
}
