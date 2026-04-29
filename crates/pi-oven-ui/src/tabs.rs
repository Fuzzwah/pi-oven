// Tab strip cells. Reads from `AppState.tabs` placeholders; real workspace
// tabs will arrive over the wire transport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::{TabCell, TabStatus};

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

    let line = build_tabs_line(tabs, inner.width as usize);
    Paragraph::new(line).render(inner, buf);
}

fn cell_spans(cell: &TabCell) -> Vec<Span<'static>> {
    let project_style = match cell.status {
        TabStatus::Active | TabStatus::Attention => {
            Style::default().add_modifier(Modifier::BOLD)
        }
        TabStatus::Idle => Style::default(),
    };
    vec![
        Span::styled(format!("[{}]", cell.project), project_style),
        Span::raw(format!(" ({})", cell.trigger)),
    ]
}

fn span_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

/// Builds the tab strip line. Cells are joined by ` > ` before an `Active`
/// (or `Attention`) cell and ` - ` before `Idle` cells. The first cell has
/// no leading separator. When the line exceeds `max_cols`, the rightmost
/// cells are dropped and a trailing ` …` is appended.
fn build_tabs_line(tabs: &[TabCell], max_cols: usize) -> Line<'static> {
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    let mut truncated = false;

    for (i, cell) in tabs.iter().enumerate() {
        let spans = cell_spans(cell);
        let cell_w = span_width(&spans);
        let sep = if i == 0 {
            None
        } else {
            Some(match cell.status {
                TabStatus::Active | TabStatus::Attention => " > ".to_string(),
                TabStatus::Idle => " - ".to_string(),
            })
        };
        let sep_w = sep.as_ref().map(|s| s.chars().count()).unwrap_or(0);
        if used + sep_w + cell_w > max_cols {
            truncated = true;
            break;
        }
        if let Some(s) = sep {
            out.push(Span::raw(s));
            used += sep_w;
        }
        out.extend(spans);
        used += cell_w;
    }

    if truncated {
        let suffix = " …";
        let suffix_w = suffix.chars().count();
        while used + suffix_w > max_cols && !out.is_empty() {
            if let Some(last) = out.pop() {
                used -= last.content.chars().count();
            }
        }
        if used > 0 {
            out.push(Span::raw(suffix.to_string()));
        } else {
            out.push(Span::raw("…".to_string()));
        }
    }

    Line::from(out)
}
