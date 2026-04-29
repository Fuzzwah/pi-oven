// Tab strip cells. Reads from `AppState.tabs` placeholders; real workspace
// tabs will arrive over the wire transport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::{TabBadge, TabCell, TabStatus};

pub fn render_tabs(area: Rect, buf: &mut Buffer, tabs: &[TabCell]) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    if tabs.is_empty() {
        Paragraph::new("(no workspaces)").render(inner, buf);
        return;
    }

    let cells: Vec<Vec<Span>> = tabs.iter().map(cell_spans).collect();
    let separator = "  ";
    let line = pack_cells(cells, separator, inner.width as usize);
    Paragraph::new(line).render(inner, buf);
}

fn cell_spans(cell: &TabCell) -> Vec<Span<'static>> {
    let dot = match cell.status {
        TabStatus::Active => "▶ ",
        TabStatus::Idle => "• ",
        TabStatus::Attention => "! ",
    };
    let dot_style = match cell.status {
        TabStatus::Active => Style::default().fg(Color::Green),
        TabStatus::Idle => Style::default().add_modifier(Modifier::DIM),
        TabStatus::Attention => Style::default().fg(Color::Yellow),
    };

    let mut spans = vec![
        Span::styled(dot.to_string(), dot_style),
        Span::raw(format!("[{}] ", cell.idx)),
        Span::styled(
            cell.project.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", cell.worktree),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ];

    if let Some(badge) = cell.badge.as_ref() {
        let (text, style) = match badge {
            TabBadge::Pr(n) => (
                format!(" #{}", n),
                Style::default().fg(Color::Magenta),
            ),
            TabBadge::Unread { up, down } => (
                format!(" ↑{} ↓{}", up, down),
                Style::default().fg(Color::Cyan),
            ),
        };
        spans.push(Span::styled(text, style));
    }

    spans
}

fn span_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

fn pack_cells(
    cells: Vec<Vec<Span<'static>>>,
    separator: &str,
    max_cols: usize,
) -> Line<'static> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    let sep_w = separator.chars().count();
    let mut truncated = false;

    for (i, cell) in cells.into_iter().enumerate() {
        let cell_w = span_width(&cell);
        let needed = if i == 0 { cell_w } else { cell_w + sep_w };
        if used + needed > max_cols {
            truncated = true;
            break;
        }
        if i > 0 {
            out.push(Span::raw(separator.to_string()));
            used += sep_w;
        }
        out.extend(cell);
        used += cell_w;
    }

    if truncated {
        // Reserve room for the ellipsis indicator; trim trailing content as needed.
        let ellipsis = "…";
        while used + 1 + sep_w > max_cols && !out.is_empty() {
            // Drop the last span (or its tail) to make room.
            if let Some(last) = out.pop() {
                used -= last.content.chars().count();
            }
        }
        if used > 0 {
            out.push(Span::raw(format!("{}{}", separator, ellipsis)));
        } else {
            out.push(Span::raw(ellipsis.to_string()));
        }
    }

    Line::from(out)
}
