//! pi-oven UI widgets and layouts. Backend-agnostic: written against the
//! generic `ratatui::backend::Backend` trait so the same widgets render under
//! both `dev-wgpu` (cell grid + glyphon) and `dev-crossterm` (terminal).

mod conversation;
mod editor;
mod header;
mod input;
mod legend;
mod sidebar;
mod status_bar;
mod tabs;

pub use conversation::render_conversation;
pub use editor::InputEditor;
pub use header::render_header;
pub use input::render_input;
pub use legend::render_legend;
pub use sidebar::render_sidebar;
pub use status_bar::render_status_bar;
pub use tabs::render_tabs;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

/// Conversation header values shown above the message body. Placeholder data
/// today; will be fed by the wire transport once it carries session metadata.
pub struct ConversationHeader {
    pub title: String,
    pub elapsed_secs: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
}

/// Bottom status-bar values. Placeholder data today; will be fed by the wire
/// transport once it carries session metadata.
pub struct StatusBar {
    pub model: String,
    pub ctx_pct: u8,
    pub branch: String,
    pub pr: Option<u32>,
}

#[derive(Clone, Copy)]
pub enum TabStatus {
    Active,
    Idle,
    Attention,
}

#[derive(Clone)]
pub enum TabBadge {
    Pr(u32),
    Unread { up: u32, down: u32 },
}

#[derive(Clone)]
pub struct TabCell {
    pub idx: u8,
    pub project: String,
    pub worktree: String,
    pub status: TabStatus,
    pub badge: Option<TabBadge>,
}

/// Application state passed into every render call.
pub struct AppState {
    pub editor: InputEditor,
    /// Whether the cursor is in its "on" phase of the blink cycle.
    pub cursor_visible: bool,
    pub header: ConversationHeader,
    pub status: StatusBar,
    pub tabs: Vec<TabCell>,
    pub legend: Vec<(String, String)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            editor: InputEditor::default(),
            cursor_visible: true,
            // MOCK: real values will arrive over the wire transport.
            header: ConversationHeader {
                title: "Apply clipboard support changes".to_string(),
                elapsed_secs: 51,
                tokens_in: 7,
                tokens_out: 2300,
            },
            // MOCK: real values will arrive over the wire transport.
            status: StatusBar {
                model: "Sonnet 4.6".to_string(),
                ctx_pct: 48,
                branch: "fuz/apply-clipboard-support".to_string(),
                pr: Some(9),
            },
            // MOCK: real workspace tabs will arrive over the wire transport.
            tabs: vec![
                TabCell {
                    idx: 1,
                    project: "website".to_string(),
                    worktree: "safe-jade".to_string(),
                    status: TabStatus::Idle,
                    badge: None,
                },
                TabCell {
                    idx: 2,
                    project: "pi-oven".to_string(),
                    worktree: "spry-mare".to_string(),
                    status: TabStatus::Active,
                    badge: Some(TabBadge::Pr(9)),
                },
                TabCell {
                    idx: 3,
                    project: "conduit".to_string(),
                    worktree: "slim-rook".to_string(),
                    status: TabStatus::Idle,
                    badge: Some(TabBadge::Pr(130)),
                },
                TabCell {
                    idx: 4,
                    project: "pi-oven".to_string(),
                    worktree: "soft-lynx".to_string(),
                    status: TabStatus::Idle,
                    badge: Some(TabBadge::Unread { up: 1, down: 2 }),
                },
            ],
            // MOCK: legend entries are descriptive only in this slice.
            legend: vec![
                ("M-tab/M-S-tab".into(), "next/prev tab".into()),
                ("C-o".into(), "model".into()),
                ("C-t".into(), "sidebar".into()),
                ("C-n".into(), "new project".into()),
                ("M-S-w".into(), "close".into()),
                ("M-S-x".into(), "archive".into()),
                ("C-c".into(), "stop".into()),
                ("C-q".into(), "quit".into()),
            ],
        }
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let [sidebar_area, right_area] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(frame.area());

    let [tabs_area, header_area, conversation_area, input_area, bottom_area] =
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .areas(right_area);

    // Bottom strip: 1 row spacing, 1 row status bar, 1 row legend.
    let [bottom_spacer, status_area, legend_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(bottom_area);
    let _ = bottom_spacer;

    let buf = frame.buffer_mut();
    render_sidebar(sidebar_area, buf);
    render_tabs(tabs_area, buf, &state.tabs);
    render_header(header_area, buf, &state.header);
    render_conversation(conversation_area, buf);
    render_input(input_area, buf, &state.editor, state.cursor_visible);
    render_status_bar(status_area, buf, &state.status);
    render_legend(legend_area, buf, &state.legend);
}
