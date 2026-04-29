use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub fn render_conversation(area: Rect, buf: &mut Buffer) {
    let block = Block::default().borders(Borders::ALL).title("Conversation");
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height > 0 && inner.width > 0 {
        Paragraph::new("(empty)")
            .alignment(Alignment::Center)
            .render(inner, buf);
    }
}
