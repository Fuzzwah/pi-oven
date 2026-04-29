use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use serde_json::Value;

use crate::AppState;

#[derive(Clone)]
pub enum RenderedEvent {
    UserMessage(String),
    /// Accumulated assistant text (all text_delta events merged into a single block).
    AssistantText(String),
    ToolCall { name: String, args_json: String },
    ToolResult { output: String, line_count: usize },
    StatusChange(String),
    RawFallback(String),
}

pub fn append_agent_event(state: &mut AppState, event: &Value) {
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");

    match event_type {
        "text_delta" => {
            let raw = event.get("text").and_then(Value::as_str).unwrap_or("");
            let text = expand_tabs(raw);
            // Accumulate into the last AssistantText block if one exists,
            // otherwise push a new one.
            match state.conversation.last_mut() {
                Some(RenderedEvent::AssistantText(buf)) => buf.push_str(&text),
                _ => state.conversation.push(RenderedEvent::AssistantText(text)),
            }
        }
        "tool_call" | "tool_use" => {
            let name = event
                .get("tool_name")
                .or_else(|| event.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let args_json = event
                .get("args")
                .or_else(|| event.get("input"))
                .map(|v| v.to_string())
                .unwrap_or_default();
            state.conversation.push(RenderedEvent::ToolCall { name, args_json });
        }
        "tool_result" => {
            let output = event
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let line_count = output.lines().count();
            state.conversation.push(RenderedEvent::ToolResult { output, line_count });
        }
        "status" | "system_prompt" => {
            let text = event
                .get("text")
                .or_else(|| event.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            state.conversation.push(RenderedEvent::StatusChange(text));
        }
        _ => {
            state.conversation.push(RenderedEvent::RawFallback(event.to_string()));
        }
    }
}

fn expand_tabs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut col: usize = 0;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = 8 - (col % 8);
            for _ in 0..spaces {
                out.push(' ');
            }
            col += spaces;
        } else {
            out.push(ch);
            if ch == '\n' {
                col = 0;
            } else {
                col += 1;
            }
        }
    }
    out
}

pub fn render_conversation(area: Rect, buf: &mut Buffer, state: &mut AppState) {
    let block = Block::default().borders(Borders::ALL).title("Conversation");
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Build a flat list of rendered lines.
    let mut lines: Vec<Line> = Vec::new();

    let accent = Style::default().fg(Color::Cyan);
    let muted = Style::default().fg(Color::DarkGray);
    let normal = Style::default();

    for event in &state.conversation {
        match event {
            RenderedEvent::UserMessage(text) => {
                for line in text.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("> ", accent),
                        Span::styled(line.to_string(), accent),
                    ]));
                }
                lines.push(Line::default());
            }
            RenderedEvent::AssistantText(text) => {
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(line.to_string(), normal)));
                }
                if !text.ends_with('\n') {
                    // already have content in last line — no extra blank needed
                } else {
                    lines.push(Line::default());
                }
            }
            RenderedEvent::ToolCall { name, .. } => {
                lines.push(Line::from(Span::styled(
                    format!("▶ {name}"),
                    muted,
                )));
            }
            RenderedEvent::ToolResult { output, line_count } => {
                const MAX_LINES: usize = 10;
                let shown: Vec<&str> = output.lines().take(MAX_LINES).collect();
                for l in &shown {
                    lines.push(Line::from(Span::styled(l.to_string(), muted)));
                }
                if *line_count > MAX_LINES {
                    lines.push(Line::from(Span::styled(
                        format!("… {} more lines", line_count - MAX_LINES),
                        muted.add_modifier(Modifier::ITALIC),
                    )));
                }
            }
            RenderedEvent::StatusChange(text) => {
                lines.push(Line::from(Span::styled(
                    format!("• {text}"),
                    muted,
                )));
            }
            RenderedEvent::RawFallback(json) => {
                lines.push(Line::from(Span::styled(json.clone(), muted)));
            }
        }
    }

    if lines.is_empty() {
        Paragraph::new("(empty)")
            .style(muted)
            .centered()
            .render(inner, buf);
        return;
    }

    let total_lines = lines.len();
    let viewport_height = inner.height as usize;

    // Task 8.3: update scroll_offset for follow mode.
    if state.follow_mode {
        state.scroll_offset = total_lines.saturating_sub(viewport_height);
    }

    let scroll_offset = state.scroll_offset.min(total_lines.saturating_sub(1));

    let text = Text::from(lines);
    Paragraph::new(text)
        .scroll((scroll_offset as u16, 0))
        .render(inner, buf);
}
