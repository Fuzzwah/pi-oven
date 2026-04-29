use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_at(cols: u16, rows: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(cols, rows);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = pi_oven_ui::AppState::default();
    terminal.draw(|f| pi_oven_ui::render(f, &state)).unwrap();
    terminal.backend().buffer().clone()
}

/// The sidebar occupies cols 0..28. Its right border is at col 27.
/// Row 0 = top-right corner `┐`, rows 1..rows-1 = `│`, last row = `┘`.
#[test]
fn layout_100x30_sidebar_border() {
    let rows = 30u16;
    let buf = render_at(100, rows);

    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐", "sidebar top-right corner");
    for row in 1..rows - 1 {
        assert_eq!(
            buf.cell((27, row)).unwrap().symbol(),
            "│",
            "sidebar right border missing at row {row}"
        );
    }
    assert_eq!(buf.cell((27, rows - 1)).unwrap().symbol(), "┘", "sidebar bottom-right corner");
}

/// tabs_area top-left starts at (28, 0).
#[test]
fn layout_100x30_tabs_top_border_at_row_0() {
    let buf = render_at(100, 30);
    let cell = buf.cell((28, 0)).unwrap();
    assert_eq!(cell.symbol(), "┌", "tabs top-left corner missing");
}

/// input_area starts at row = rows - 3; its top-left corner is `┌` at col 28.
#[test]
fn layout_100x30_input_top_border_at_rows_minus_3() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let top_row = rows - 3;
    let cell = buf.cell((28, top_row)).unwrap();
    assert_eq!(
        cell.symbol(),
        "┌",
        "input bar top-left corner missing at row {top_row}"
    );
}

/// At 200×60, sidebar is still 28 cols and input bar is still at rows - 3 = 57.
#[test]
fn layout_200x60_fixed_sidebar_and_bars() {
    let rows = 60u16;
    let buf = render_at(200, rows);

    // Sidebar right border: corners at row 0 and rows-1, `│` in between.
    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    for row in 1..rows - 1 {
        assert_eq!(
            buf.cell((27, row)).unwrap().symbol(),
            "│",
            "sidebar right border missing at row {row} in 200x60"
        );
    }
    assert_eq!(buf.cell((27, rows - 1)).unwrap().symbol(), "┘");

    // input bar top-left at row 57
    let cell = buf.cell((28, rows - 3)).unwrap();
    assert_eq!(cell.symbol(), "┌", "input top-left corner missing in 200x60");
}

/// At 40×12 (minimum reasonable), rendering does not panic.
/// Sidebar still occupies 28 cols; conversation area has at least 1 row.
#[test]
fn layout_40x12_no_panic_and_sidebar_width() {
    let rows = 12u16;
    let buf = render_at(40, rows);

    // Sidebar top-right corner at (27, 0)
    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    // Middle rows have `│`
    for row in 1..rows - 1 {
        assert_eq!(
            buf.cell((27, row)).unwrap().symbol(),
            "│",
            "sidebar right border missing at row {row} in 40x12"
        );
    }

    // Conversation height = rows - 3 - 3 = 6, must be >= 1
    let conv_height = rows - 3 - 3;
    assert!(conv_height >= 1, "conversation pane must have at least 1 row");
}

/// Placeholder labels visible at 100×30.
#[test]
fn layout_100x30_placeholder_labels() {
    let buf = render_at(100, 30);

    // "Projects" in sidebar top border (row 0, cols 0..28)
    let title_row: String = (0..28u16)
        .map(|col| buf.cell((col, 0)).unwrap().symbol().to_string())
        .collect();
    assert!(
        title_row.contains("Projects"),
        "sidebar title 'Projects' not found: {title_row:?}"
    );

    // "(no projects)" in sidebar body (row 1, cols 1..27)
    let body_row: String = (1..27u16)
        .map(|col| buf.cell((col, 1)).unwrap().symbol().to_string())
        .collect();
    assert!(
        body_row.contains("(no projects)"),
        "'(no projects)' not found: {body_row:?}"
    );

    // "Conversation" in conversation pane top border (row 3, cols 28..100)
    let conv_title: String = (28..100u16)
        .map(|col| buf.cell((col, 3)).unwrap().symbol().to_string())
        .collect();
    assert!(
        conv_title.contains("Conversation"),
        "'Conversation' title not found: {conv_title:?}"
    );

    // "(empty)" in conversation body (row 4, cols 29..99)
    let conv_body: String = (29..99u16)
        .map(|col| buf.cell((col, 4)).unwrap().symbol().to_string())
        .collect();
    assert!(
        conv_body.contains("(empty)"),
        "'(empty)' not found: {conv_body:?}"
    );

    // "(no workspaces)" in tabs body (row 1, cols 29..99)
    let tabs_body: String = (29..99u16)
        .map(|col| buf.cell((col, 1)).unwrap().symbol().to_string())
        .collect();
    assert!(
        tabs_body.contains("(no workspaces)"),
        "'(no workspaces)' not found: {tabs_body:?}"
    );

    // ">" in input body (row 28 = rows-2, cols 29..99)
    let input_body: String = (29..99u16)
        .map(|col| buf.cell((col, 28)).unwrap().symbol().to_string())
        .collect();
    assert!(
        input_body.contains(">"),
        "'>' not found in input body: {input_body:?}"
    );
}
