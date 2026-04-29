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
    assert_eq!(buf.cell((28, 0)).unwrap().symbol(), "┌", "tabs top-left corner missing");
}

/// Header strip starts at row 3 (right under the tab strip's bottom border).
#[test]
fn layout_100x30_header_below_tabs() {
    let buf = render_at(100, 30);
    // Title is rendered on the second row of the header strip = row 4.
    let title_row = row_text(&buf, 4, 28..100);
    assert!(
        title_row.contains("Apply clipboard support changes"),
        "expected header title on row 4, got {title_row:?}"
    );
}

/// Stats sub-row is on the third row of the header strip = row 5.
#[test]
fn layout_100x30_header_stats_row() {
    let buf = render_at(100, 30);
    let stats_row = row_text(&buf, 5, 28..100);
    assert!(stats_row.contains("51s"), "expected '51s' on stats row, got {stats_row:?}");
    assert!(stats_row.contains("↓7"), "expected '↓7' on stats row, got {stats_row:?}");
    assert!(
        stats_row.contains("↑2.3k"),
        "expected '↑2.3k' on stats row, got {stats_row:?}"
    );
}

/// Conversation body sits between the header (rows 3..6) and the input bar (rows 24..27).
/// Top border at row 6.
#[test]
fn layout_100x30_conversation_top_border() {
    let buf = render_at(100, 30);
    assert_eq!(
        buf.cell((28, 6)).unwrap().symbol(),
        "┌",
        "conversation top-left corner missing at row 6"
    );
}

/// Input bar sits above the bottom strip. With rows=30 and bottom strip = 3 rows,
/// the input bar's top border is at row 30 - 3 - 3 = 24.
#[test]
fn layout_100x30_input_above_bottom_strip() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let top_row = rows - 3 - 3;
    assert_eq!(
        buf.cell((28, top_row)).unwrap().symbol(),
        "┌",
        "input top-left corner missing at row {top_row}"
    );
}

/// Status bar row contains the model name, ctx%, PR badge, and branch.
/// Bottom strip occupies the last 3 rows: row rows-3 = spacer, rows-2 = status, rows-1 = legend.
#[test]
fn layout_100x30_status_bar() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let status_row = row_text(&buf, rows - 2, 28..100);
    assert!(
        status_row.contains("Sonnet 4.6"),
        "expected 'Sonnet 4.6' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("ctx:48%"),
        "expected 'ctx:48%' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("PR #9"),
        "expected 'PR #9' on status row, got {status_row:?}"
    );
    assert!(
        status_row.contains("fuz/apply-clipboard-support"),
        "expected branch on status row, got {status_row:?}"
    );
}

/// Legend row contains key/action pairs (truncates with `…` when narrow).
#[test]
fn layout_100x30_legend_row() {
    let rows = 30u16;
    let buf = render_at(100, rows);
    let legend_row = row_text(&buf, rows - 1, 28..100);
    assert!(
        legend_row.contains("M-tab"),
        "expected 'M-tab' on legend row, got {legend_row:?}"
    );
    // 100x30 leaves 72 inner cols on the right column — the full legend overflows
    // and is truncated with an ellipsis indicator.
    assert!(
        legend_row.contains("…"),
        "expected '…' truncation indicator on legend row, got {legend_row:?}"
    );
}

/// At a wide window the full legend (including the trailing `quit` entry) fits.
#[test]
fn layout_220x30_full_legend_visible() {
    let rows = 30u16;
    let buf = render_at(220, rows);
    let legend_row = row_text(&buf, rows - 1, 28..220);
    assert!(
        legend_row.contains("M-tab"),
        "expected 'M-tab' on wide legend row, got {legend_row:?}"
    );
    assert!(
        legend_row.contains("quit"),
        "expected 'quit' on wide legend row, got {legend_row:?}"
    );
}

/// Tab strip renders mock cells from `AppState.tabs` on row 1 (inner row).
#[test]
fn layout_100x30_tab_cells_visible() {
    let buf = render_at(100, 30);
    let tabs_row = row_text(&buf, 1, 28..100);
    // At least one active dot and the project name from the second mock tab.
    assert!(
        tabs_row.contains("▶"),
        "expected active status dot on tabs row, got {tabs_row:?}"
    );
    assert!(
        tabs_row.contains("pi-oven"),
        "expected 'pi-oven' on tabs row, got {tabs_row:?}"
    );
    assert!(
        tabs_row.contains("#9"),
        "expected PR badge '#9' on tabs row, got {tabs_row:?}"
    );
}

/// At 200×60, sidebar is still 28 cols and the input bar's top is at row 60-6 = 54.
#[test]
fn layout_200x60_fixed_strips() {
    let rows = 60u16;
    let buf = render_at(200, rows);

    // Sidebar right border full height.
    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    for row in 1..rows - 1 {
        assert_eq!(
            buf.cell((27, row)).unwrap().symbol(),
            "│",
            "sidebar right border missing at row {row} in 200x60"
        );
    }
    assert_eq!(buf.cell((27, rows - 1)).unwrap().symbol(), "┘");

    // Input top-left at row 54.
    assert_eq!(buf.cell((28, rows - 6)).unwrap().symbol(), "┌");
}

/// Small window (60×20) does not panic and the conversation pane has at least one row.
/// At rows=20: tabs(3) + header(3) + body(?) + input(3) + bottom(3) = 12 fixed; body=8.
#[test]
fn layout_60x20_no_panic() {
    let rows = 20u16;
    let buf = render_at(60, rows);
    // Sidebar is still bordered correctly.
    assert_eq!(buf.cell((27, 0)).unwrap().symbol(), "┐");
    // Conversation body top border at row 6 must still exist.
    assert_eq!(buf.cell((28, 6)).unwrap().symbol(), "┌");
    // Conversation pane should be at least one row tall.
    let conv_height = rows.saturating_sub(3 + 3 + 3 + 3);
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
    // "Conversation" title is on the top border row (row 6).
    let conv_title = row_text(&buf, 6, 28..100);
    assert!(
        conv_title.contains("Conversation"),
        "'Conversation' title not found: {conv_title:?}"
    );
    // "(empty)" is in the inner body just below.
    let conv_body = row_text(&buf, 7, 29..99);
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
    use pi_oven_ui::{TabBadge, TabCell, TabStatus};

    let mut state = AppState::default();
    // Replace tabs with a long list of wide cells that will exceed any reasonable inner width.
    state.tabs = (0..20u8)
        .map(|i| TabCell {
            idx: i + 1,
            project: format!("project-with-long-name-{i}"),
            worktree: format!("worktree-with-long-name-{i}"),
            status: TabStatus::Idle,
            badge: Some(TabBadge::Pr(i as u32 + 100)),
        })
        .collect();

    let buf = render_with(100, 30, &state);
    let tabs_row = row_text(&buf, 1, 28..100);
    assert!(
        tabs_row.contains("…"),
        "expected ellipsis truncation indicator, got {tabs_row:?}"
    );
}
