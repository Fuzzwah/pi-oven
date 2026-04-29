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

pub use conversation::{append_agent_event, render_conversation, RenderedEvent};
pub use editor::InputEditor;
pub use header::render_header;
pub use input::render_input;
pub use legend::render_legend;
pub use sidebar::render_sidebar;
pub use status_bar::render_status_bar;
pub use tabs::render_tabs;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

/// Conversation header — currently just a title. Placeholder data today;
/// will be fed by the wire transport once it carries session metadata.
pub struct ConversationHeader {
    pub title: String,
}

/// Bottom status-bar values. Free-form strings so the demo can show generic
/// placeholder text; the wire-transport slice will tighten the types.
pub struct StatusBar {
    pub model: String,
    pub ctx: String,
    pub branch: String,
    pub pr: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum AgentStatusKind {
    Running,
    Idle,
}

#[derive(Clone, Copy)]
pub enum TabStatus {
    Active,
    Idle,
    Attention,
}

#[derive(Clone)]
pub struct TabCell {
    pub idx: u8,
    pub project: String,
    pub trigger: String,
    pub status: TabStatus,
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
    /// Live session conversation buffer.
    pub conversation: Vec<RenderedEvent>,
    /// Lines scrolled from the top of the conversation.
    pub scroll_offset: usize,
    /// When true, viewport auto-scrolls to follow new events.
    pub follow_mode: bool,
    /// Current status of the active workspace session.
    pub workspace_status: AgentStatusKind,
    /// Highest seq received from the server (for Resume on reconnect).
    pub last_seq: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            editor: InputEditor::default(),
            cursor_visible: true,
            // MOCK: real values will arrive over the wire transport.
            header: ConversationHeader {
                title: "[project 1] Longer Explanation of Feature xyz".to_string(),
            },
            // MOCK: real values will arrive over the wire transport.
            status: StatusBar {
                model: "[Model]".to_string(),
                ctx: "[context %]".to_string(),
                branch: "[branch name]".to_string(),
                pr: Some("[123]".to_string()),
            },
            // MOCK: real workspace tabs will arrive over the wire transport.
            tabs: vec![
                TabCell {
                    idx: 1,
                    project: "project 1".to_string(),
                    trigger: "issue-123".to_string(),
                    status: TabStatus::Idle,
                },
                TabCell {
                    idx: 2,
                    project: "project 1".to_string(),
                    trigger: "spec-feat-xyz".to_string(),
                    status: TabStatus::Active,
                },
                TabCell {
                    idx: 3,
                    project: "project 2".to_string(),
                    trigger: "spec-add-juice".to_string(),
                    status: TabStatus::Idle,
                },
                TabCell {
                    idx: 4,
                    project: "project 2".to_string(),
                    trigger: "exp-test".to_string(),
                    status: TabStatus::Idle,
                },
            ],
            // Real hotkeys actually wired in `pi-oven/src/main.rs`.
            legend: vec![
                ("Cmd+W".into(), "quit".into()),
                ("Cmd+C".into(), "copy".into()),
                ("Cmd+X".into(), "cut".into()),
                ("Cmd+V".into(), "paste".into()),
                ("Cmd+=/-".into(), "font size".into()),
            ],
            conversation: Vec::new(),
            scroll_offset: 0,
            follow_mode: true,
            workspace_status: AgentStatusKind::Idle,
            last_seq: 0,
        }
    }
}


pub fn render(frame: &mut Frame, state: &mut AppState) {
    let [sidebar_area, right_area] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(frame.area());

    let [tabs_area, header_area, conversation_area, input_area, bottom_area] =
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .areas(right_area);

    // Bottom strip: 1 row status bar, 1 row legend (no leading spacer).
    let [status_area, legend_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(bottom_area);

    let buf = frame.buffer_mut();
    render_sidebar(sidebar_area, buf);
    render_tabs(tabs_area, buf, &state.tabs);
    render_header(header_area, buf, &state.header);
    render_conversation(conversation_area, buf, state);
    render_input(input_area, buf, &state.editor, state.cursor_visible);
    render_status_bar(status_area, buf, &state.status);
    render_legend(legend_area, buf, &state.legend);
}
