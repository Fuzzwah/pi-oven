use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub fn render_input(area: Rect, buf: &mut Buffer) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    block.render(area, buf);
    if inner.height > 0 && inner.width > 0 {
        Paragraph::new(">").render(inner, buf);
    }
}
