use pi_oven_render::{RatatuiGridBackend, Grid};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

#[test]
fn paragraph_writes_into_grid_at_the_layout_position() {
    let backend = RatatuiGridBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, f.area().width, 1);
            f.render_widget(Paragraph::new("pi-oven"), area);
        })
        .expect("draw");

    let grid: &Grid = terminal.backend().grid();
    let row0: String = (0..7)
        .map(|x| grid.get(x, 0).expect("cell").symbol.clone())
        .collect();
    assert_eq!(row0, "pi-oven");
}

#[test]
fn styled_paragraph_preserves_fg_in_grid() {
    let backend = RatatuiGridBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, f.area().width, 1);
            f.render_widget(
                Paragraph::new("X").style(ratatui::style::Style::default().fg(Color::Red)),
                area,
            );
        })
        .expect("draw");

    let grid: &Grid = terminal.backend().grid();
    let cell = grid.get(0, 0).expect("cell");
    assert_eq!(cell.symbol, "X");
    assert_eq!(cell.fg, Color::Red);
}

#[test]
fn multi_codepoint_symbols_are_preserved_in_grid() {
    let backend = RatatuiGridBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, f.area().width, 1);
            f.render_widget(Paragraph::new("e\u{301}"), area);
        })
        .expect("draw");

    let grid: &Grid = terminal.backend().grid();
    let cell = grid.get(0, 0).expect("cell");
    assert_eq!(cell.symbol, "e\u{301}");
}

#[test]
fn styled_cells_preserve_background_and_underline_in_grid() {
    let backend = RatatuiGridBackend::new(20, 5);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|f| {
            let area = Rect::new(0, 0, f.area().width, 1);
            f.render_widget(
                Paragraph::new("X").style(
                    Style::default()
                        .fg(Color::Red)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                area,
            );
        })
        .expect("draw");

    let grid: &Grid = terminal.backend().grid();
    let cell = grid.get(0, 0).expect("cell");
    assert_eq!(cell.bg, Color::Blue);
    assert!(cell.attrs.contains(Modifier::UNDERLINED));
}
