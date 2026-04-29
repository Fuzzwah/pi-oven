//! pi-oven UI widgets and layouts. Backend-agnostic: written against the
//! generic `ratatui::backend::Backend` trait so the same widgets render under
//! both `dev-wgpu` (cell grid + glyphon) and `dev-crossterm` (terminal).

mod conversation;
mod editor;
mod input;
mod sidebar;
mod tabs;

pub use conversation::render_conversation;
pub use editor::InputEditor;
pub use input::render_input;
pub use sidebar::render_sidebar;
pub use tabs::render_tabs;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

/// Application state passed into every render call.
pub struct AppState {
    pub editor: InputEditor,
    /// Whether the cursor is in its "on" phase of the blink cycle.
    pub cursor_visible: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self { editor: InputEditor::default(), cursor_visible: true }
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let [sidebar_area, right_area] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(frame.area());

    let [tabs_area, conversation_area, input_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .areas(right_area);

    let buf = frame.buffer_mut();
    render_sidebar(sidebar_area, buf);
    render_tabs(tabs_area, buf);
    render_conversation(conversation_area, buf);
    render_input(input_area, buf, &state.editor, state.cursor_visible);
}
