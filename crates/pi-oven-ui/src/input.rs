use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::InputEditor;

pub fn render_input(area: Rect, buf: &mut Buffer, editor: &InputEditor, cursor_visible: bool) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let text = editor.text();
    let cursor = editor.cursor_byte_pos();
    let selection = editor.selection();

    let reversed = Style::default().add_modifier(Modifier::REVERSED);
    let mut spans: Vec<Span> = vec![Span::raw("> ")];

    if let Some((sel_start, sel_end)) = selection {
        if sel_start > 0 {
            spans.push(Span::raw(&text[..sel_start]));
        }
        if sel_end > sel_start {
            spans.push(Span::styled(&text[sel_start..sel_end], reversed));
        }
        if sel_end < text.len() {
            spans.push(Span::raw(&text[sel_end..]));
        }
    } else if cursor_visible {
        if cursor > 0 {
            spans.push(Span::raw(&text[..cursor]));
        }
        if cursor < text.len() {
            let ch = text[cursor..].chars().next().unwrap();
            let end = cursor + ch.len_utf8();
            spans.push(Span::styled(&text[cursor..end], reversed));
            if end < text.len() {
                spans.push(Span::raw(&text[end..]));
            }
        } else {
            spans.push(Span::styled(" ", reversed));
        }
    } else {
        spans.push(Span::raw(text));
    }

    Paragraph::new(Line::from(spans)).render(inner, buf);
}
