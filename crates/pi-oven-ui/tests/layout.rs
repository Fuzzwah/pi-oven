use pi_oven_ui::AppState;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_with(cols: u16, rows: u16, state: &AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(cols, rows);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| pi_oven_ui::render(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn render_at(cols: u16, rows: u16) -> ratatui::buffer::Buffer {
    render_with(cols, rows, &AppState::default())
}

fn row_text(buf: &ratatui::buffer::Buffer, row: u16, cols: std::ops::Range<u16>) -> String {
    cols.map(|col| buf.cell((col, row)).unwrap().symbol().to_string()).collect()
}

/// Sidebar occupies cols 0..28; right border at col 27.
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

/// tabs_area top-left at (28, 0).
#[test]
fn layout_100x30_tabs_top_border_at_row_0() {
    let buf = render_at(100, 30);
    assert_eq!(buf.cell((28, 0)).unwrap().symbol(), "┌", "tabs top-left corner missing");
}

/// Header strip is row 3 (one row, no spacer); the title is centered + bold there.
#[test]
fn layout_100x30_header_directly_below_tabs() {
    let buf = render_at(100, 30);
    let header_row = row_text(&buf, 3, 28..100);
    assert!(
        header_row.contains("[project 1] Longer Explanation of Feature xyz"),
        "expected centered bold title on row 3, got {header_row:?}"
    );
}

/// Conversation body top border at row 4 (immediately below the 1-row header).
#[test]
fn layout_100x30_conversation_top_border() {
    let buf = render_at(100, 30);
    assert_eq!(
        buf.cell((28, 4)).unwrap().symbol(),
        "┌",
        "conversation top-left corner missing at row 4"
    );
}

/// Input bar sits directly above the 2-row bottom strip. With rows=30,
/// input top border is at row 30 - 2 - 3 = 25.
#[test]
fn layout_100x30_input_above_bottom_strip() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let top_row = rows - 2 - 3;
    assert_eq!(
        buf.cell((28, top_row)).unwrap().symbol(),
        "┌",
        "input top-left corner missing at row {top_row}"
    );
}

/// Status bar row contains model, context, PR#, and branch joined with ` - `.
/// Bottom strip occupies the last 2 rows: row rows-2 = status, row rows-1 = legend.
#[test]
fn layout_100x30_status_bar() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let status_row = row_text(&buf, rows - 2, 28..100);
    assert!(
        status_row.contains("[Model]"),
        "expected '[Model]' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("[context %]"),
        "expected '[context %]' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("PR# [123]"),
        "expected 'PR# [123]' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("[branch name]"),
        "expected '[branch name]' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains(" - "),
        "expected ' - ' separator on status row, got {status_row:?}"
    );
}

/// Legend row contains real hotkeys.
#[test]
fn layout_100x30_legend_row() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let legend_row = row_text(&buf, rows - 1, 28..100);
    assert!(
        legend_row.contains("Cmd+W"),
        "expected 'Cmd+W' on legend row, got {legend_row:?}"
    );
    assert!(
        legend_row.contains("quit"),
        "expected 'quit' on legend row, got {legend_row:?}"
    );
    assert!(
        legend_row.contains("Cmd+C"),
        "expected 'Cmd+C' on legend row, got {legend_row:?}"
    );
}

/// Tab strip renders mock cells with `[project N] (trigger)` form and
/// `>` before the active tab, `-` between idle tabs. Uses 150 cols so the
/// full mock list fits without truncation.
#[test]
fn layout_150x30_tab_cells_visible() {
    let buf = render_at(150, 30);
    let tabs_row = row_text(&buf, 1, 28..150);
    assert!(
        tabs_row.contains("[project 1] (issue-123)"),
        "expected first tab cell, got {tabs_row:?}"
    );
    assert!(
        tabs_row.contains(" > [project 1] (spec-feat-xyz)"),
        "expected '>' separator before active cell, got {tabs_row:?}"
    );
    assert!(
        tabs_row.contains(" - [project 2] (spec-add-juice)"),
        "expected '-' separator between idle cells, got {tabs_row:?}"
    );
    assert!(
        tabs_row.contains(" - [project 2] (exp-test)"),
        "expected last tab cell separated by '-', got {tabs_row:?}"
    );
}

/// At 200×60, sidebar is still 28 cols and the input bar's top is at row 60-5 = 55.
#[test]
fn layout_200x60_fixed_strips() {
    let rows = 60u16;
    let buf = render_at(200, rows);

    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    for row in 1..rows - 1 {
        assert_eq!(
            buf.cell((27, row)).unwrap().symbol(),
            "│",
            "sidebar right border missing at row {row} in 200x60"
        );
    }
    assert_eq!(buf.cell((27, rows - 1)).unwrap().symbol(), "┘");

    // Input top-left at row rows - 2 - 3 = 55.
    assert_eq!(buf.cell((28, rows - 5)).unwrap().symbol(), "┌");
}

/// Small window (60×20) does not panic and the conversation pane has at least one row.
/// Fixed strips total 3 + 1 + 3 + 2 = 9; conversation gets 11 at rows=20.
#[test]
fn layout_60x20_no_panic() {
    let rows = 20u16;
    let buf = render_at(60, rows);
    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    // Conversation body top border at row 4.
    assert_eq!(buf.cell((28, 4)).unwrap().symbol(), "┌");
    let conv_height = rows.saturating_sub(3 + 1 + 3 + 2);
    assert!(conv_height >= 1, "conversation pane must have at least 1 row");
}

/// Sidebar placeholder label still rendered.
#[test]
fn layout_100x30_sidebar_placeholder() {
    let buf = render_at(100, 30);
    let title_row = row_text(&buf, 0, 0..28);
    assert!(title_row.contains("Projects"), "sidebar title missing: {title_row:?}");
    let body_row = row_text(&buf, 1, 1..27);
    assert!(body_row.contains("(no projects)"), "sidebar body missing: {body_row:?}");
}

/// Conversation body still shows `(empty)` placeholder.
#[test]
fn layout_100x30_conversation_placeholder() {
    let buf = render_at(100, 30);
    // "Conversation" title is on the top border row (row 4).
    let conv_title = row_text(&buf, 4, 28..100);
    assert!(
        conv_title.contains("Conversation"),
        "'Conversation' title not found: {conv_title:?}"
    );
    // "(empty)" is in the inner body just below.
    let conv_body = row_text(&buf, 5, 29..99);
    assert!(
        conv_body.contains("(empty)"),
        "'(empty)' not found: {conv_body:?}"
    );
}

/// Empty `tabs` falls back to the `(no workspaces)` placeholder.
#[test]
fn layout_100x30_empty_tabs_placeholder() {
    let mut state = AppState::default();
    state.tabs.clear();
    let buf = render_with(100, 30, &state);
    let tabs_body = row_text(&buf, 1, 29..99);
    assert!(
        tabs_body.contains("(no workspaces)"),
        "'(no workspaces)' not found: {tabs_body:?}"
    );
}

/// When tab cells are wider than the area, the rightmost cells truncate with `…`.
#[test]
fn tabs_truncate_overflow() {
    use pi_oven_ui::{TabCell, TabStatus};

    let mut state = AppState::default();
    state.tabs = (0..20u8)
        .map(|i| TabCell {
            idx: i + 1,
            project: format!("project-with-long-name-{i}"),
            trigger: format!("trigger-with-long-name-{i}"),
            status: TabStatus::Idle,
        })
        .collect();

    let buf = render_with(100, 30, &state);
    let tabs_row = row_text(&buf, 1, 28..100);
    assert!(
        tabs_row.contains("…"),
        "expected ellipsis truncation indicator, got {tabs_row:?}"
    );
}
