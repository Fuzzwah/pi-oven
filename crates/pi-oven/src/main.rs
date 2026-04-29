// Mutually-exclusive feature flags — see `pi-oven/Cargo.toml`.
#[cfg(all(feature = "dev-wgpu", feature = "dev-crossterm"))]
compile_error!(
    "features `dev-wgpu` and `dev-crossterm` are mutually exclusive; \
     pass --no-default-features --features dev-crossterm to use the terminal backend"
);

#[cfg(not(any(feature = "dev-wgpu", feature = "dev-crossterm")))]
compile_error!(
    "exactly one of `dev-wgpu` (default) or `dev-crossterm` must be enabled"
);

#[cfg(feature = "dev-wgpu")]
mod keys;
mod config;

const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,cosmic_text=error")),
        )
        .init();
}

struct NetChannels {
    /// Send outgoing `Msg` to the server.
    out_tx: tokio::sync::mpsc::Sender<pi_oven_protocol::Msg>,
    /// Receive incoming `Msg` from the server (std channel, poll via try_recv).
    in_rx: std::sync::mpsc::Receiver<pi_oven_protocol::Msg>,
}

/// Spawn the tokio runtime on a dedicated OS thread and start the reconnecting
/// WebSocket client.
fn spawn_network(
    net_event_tx: std::sync::mpsc::Sender<pi_oven_net::ConnectionState>,
) -> Option<NetChannels> {
    let url = std::env::var("PI_OVEN_SERVER_URL")
        .unwrap_or_else(|_| "ws://localhost:7878".to_string());
    let key = match std::env::var("PI_OVEN_SHARED_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::warn!(
                "PI_OVEN_SHARED_KEY not set — running without networking (UI-only mode)"
            );
            return None;
        }
    };

    // tokio channel for outgoing messages (UI → server).
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<pi_oven_protocol::Msg>(64);
    // std channel for incoming messages (server → UI).
    let (std_in_tx, std_in_rx) = std::sync::mpsc::channel::<pi_oven_protocol::Msg>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let handle = pi_oven_net::start_reconnecting(url, key, CLIENT_VERSION.to_string());
            let reconnect_msg_tx = handle.msg_tx;
            let mut reconnect_msg_rx = handle.msg_rx;
            let mut state_rx = handle.state_rx;

            // Forward outgoing messages from UI to the reconnect handle.
            tokio::spawn(async move {
                while let Some(msg) = out_rx.recv().await {
                    let _ = reconnect_msg_tx.send(msg).await;
                }
            });

            // Forward incoming messages from reconnect to the UI (std channel).
            tokio::spawn(async move {
                while let Some(msg) = reconnect_msg_rx.recv().await {
                    let _ = std_in_tx.send(msg);
                }
            });

            // Forward connection state changes to the UI.
            loop {
                let state = state_rx.borrow().clone();
                match &state {
                    pi_oven_net::ConnectionState::Connecting => {
                        tracing::info!("net: connecting");
                    }
                    pi_oven_net::ConnectionState::Authenticated => {
                        tracing::info!("net: authenticated");
                    }
                    pi_oven_net::ConnectionState::Reconnecting { in_seconds } => {
                        tracing::info!(in_seconds, "net: reconnecting");
                    }
                    pi_oven_net::ConnectionState::Failed { reason } => {
                        tracing::info!(%reason, "net: disconnected (terminal)");
                    }
                }
                if net_event_tx.send(state).is_err() {
                    break;
                }
                if state_rx.changed().await.is_err() {
                    break;
                }
            }
        });
    });

    Some(NetChannels { out_tx, in_rx: std_in_rx })
}

#[cfg(feature = "dev-wgpu")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-wgpu)");
    let (net_tx, _net_rx) = std::sync::mpsc::channel::<pi_oven_net::ConnectionState>();
    let net = spawn_network(net_tx);
    wgpu_main::run(net)
}

#[cfg(feature = "dev-crossterm")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-crossterm)");
    let (net_tx, _net_rx) = std::sync::mpsc::channel::<pi_oven_net::ConnectionState>();
    let net = spawn_network(net_tx);
    crossterm_main::run(net)
}

// =============================================================================
// dev-wgpu: native winit window + wgpu/glyphon paint via pi-oven-render
// =============================================================================

#[cfg(feature = "dev-wgpu")]
mod wgpu_main {
    use std::sync::Arc;

    use anyhow::Result;
    use pi_oven_protocol::Msg;
    use pi_oven_render::{Painter, RatatuiGridBackend};
    use pi_oven_ui::{append_agent_event, AgentStatusKind, RenderedEvent};
    use ratatui::Terminal;
    use std::time::{Duration, Instant};

    use winit::application::ApplicationHandler;
    use winit::event::{KeyEvent, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{Key, ModifiersState, NamedKey};
    use winit::window::{Window, WindowAttributes, WindowId};

    use crate::keys::{translate, KeyAction};

    const FONT_SIZE_PX: f32 = 18.0;
    const FONT_SIZE_STEP: f32 = 2.0;
    const FONT_SIZE_MIN: f32 = 12.0;
    const FONT_SIZE_MAX: f32 = 48.0;
    const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(530);
    const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);
    const FONT_SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

    struct App {
        window: Option<Arc<Window>>,
        painter: Option<Painter>,
        terminal: Option<Terminal<RatatuiGridBackend>>,
        modifiers: ModifiersState,
        font_size: f32,
        app_state: pi_oven_ui::AppState,
        next_blink: Instant,
        last_resize: Option<Instant>,
        font_save_pending: Option<Instant>,
        net_out: Option<tokio::sync::mpsc::Sender<Msg>>,
        net_in: Option<std::sync::mpsc::Receiver<Msg>>,
    }

    impl App {
        fn new(net: Option<crate::NetChannels>) -> Self {
            let (net_out, net_in) = match net {
                Some(ch) => (Some(ch.out_tx), Some(ch.in_rx)),
                None => (None, None),
            };
            Self {
                window: None,
                painter: None,
                terminal: None,
                modifiers: ModifiersState::empty(),
                font_size: crate::config::load_font_size(FONT_SIZE_PX)
                    .clamp(FONT_SIZE_MIN, FONT_SIZE_MAX),
                app_state: pi_oven_ui::AppState::default(),
                next_blink: Instant::now() + CURSOR_BLINK_INTERVAL,
                last_resize: None,
                font_save_pending: None,
                net_out,
                net_in,
            }
        }

        fn reset_blink(&mut self) {
            self.app_state.cursor_visible = true;
            self.next_blink = Instant::now() + CURSOR_BLINK_INTERVAL;
        }

        fn rebuild_terminal(&mut self) {
            if let Some(painter) = self.painter.as_ref() {
                let (cols, rows) = painter.grid_dimensions();
                let backend = RatatuiGridBackend::new(cols, rows);
                match Terminal::new(backend) {
                    Ok(t) => self.terminal = Some(t),
                    Err(e) => tracing::error!(?e, "failed to construct ratatui Terminal"),
                }
            }
        }

        fn redraw(&mut self) {
            let (Some(painter), Some(terminal)) =
                (self.painter.as_mut(), self.terminal.as_mut())
            else {
                return;
            };
            let state = &mut self.app_state;
            if let Err(e) = terminal.draw(|f| {
                pi_oven_ui::render(f, state);
            }) {
                tracing::error!(?e, "ratatui draw failed");
                return;
            }
            let dirty = terminal.backend_mut().take_dirty_rows();
            if let Err(e) = painter.paint(terminal.backend().grid(), &dirty) {
                tracing::error!(?e, "wgpu paint failed");
            }
        }

        fn send_msg(&self, msg: Msg) {
            if let Some(tx) = &self.net_out {
                let _ = tx.blocking_send(msg);
            }
        }

        fn handle_net_messages(&mut self) -> bool {
            let msgs: Vec<pi_oven_protocol::Msg> = {
                let Some(rx) = &self.net_in else { return false };
                let mut msgs = Vec::new();
                loop {
                    match rx.try_recv() {
                        Ok(msg) => msgs.push(msg),
                        Err(_) => break,
                    }
                }
                msgs
            };
            if msgs.is_empty() {
                return false;
            }
            for msg in msgs {
                self.process_msg(msg);
            }
            true
        }

        fn process_msg(&mut self, msg: Msg) {
            match msg {
                Msg::Welcome { workspaces, .. } => {
                    for ws in &workspaces {
                        self.send_msg(Msg::Resume {
                            workspace_id: ws.workspace_id,
                            last_seq: self.app_state.last_seq,
                        });
                    }
                }
                Msg::ReplayBatch { events, latest_seq, .. } => {
                    for entry in &events {
                        append_agent_event(
                            &mut self.app_state,
                            &entry.event,
                        );
                    }
                    self.app_state.last_seq = latest_seq;
                }
                Msg::AgentEvent { seq, event, .. } => {
                    append_agent_event(&mut self.app_state, &event);
                    self.app_state.last_seq = seq;
                }
                Msg::AgentStatus { status, .. } => {
                    self.app_state.workspace_status = if status == "running" {
                        AgentStatusKind::Running
                    } else {
                        AgentStatusKind::Idle
                    };
                }
                _ => {}
            }
        }
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = WindowAttributes::default()
                .with_title("pi-oven")
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
            let window = match event_loop.create_window(attrs) {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    tracing::error!(?e, "create_window failed");
                    event_loop.exit();
                    return;
                }
            };
            self.window = Some(window.clone());

            match pollster::block_on(Painter::new(window.clone(), self.font_size)) {
                Ok(p) => {
                    self.painter = Some(p);
                    self.rebuild_terminal();
                }
                Err(e) => {
                    tracing::error!(?e, "Painter::new failed");
                    event_loop.exit();
                }
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    tracing::info!("close requested");
                    self.flush_font_save();
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    if let Some(p) = self.painter.as_mut() {
                        p.resize_surface_only(size);
                    }
                    self.last_resize = Some(Instant::now());
                    if let Some(w) = self.window.as_ref() {
                        w.request_redraw();
                    }
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    if let Some(p) = self.painter.as_mut() {
                        p.scale_factor_changed(scale_factor);
                    }
                }
                WindowEvent::ModifiersChanged(m) => {
                    self.modifiers = m.state();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    self.handle_key(event_loop, event);
                }
                WindowEvent::RedrawRequested => {
                    self.redraw();
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            let now = Instant::now();
            let mut redraw_needed = false;

            if now >= self.next_blink {
                self.app_state.cursor_visible = !self.app_state.cursor_visible;
                self.next_blink = now + CURSOR_BLINK_INTERVAL;
                redraw_needed = true;
            }

            if let Some(t) = self.last_resize {
                if now.duration_since(t) >= RESIZE_DEBOUNCE {
                    if let Some(p) = self.painter.as_mut() {
                        p.rebuild_layout();
                    }
                    self.rebuild_terminal();
                    self.last_resize = None;
                    redraw_needed = true;
                }
            }

            if let Some(t) = self.font_save_pending {
                if now.duration_since(t) >= FONT_SAVE_DEBOUNCE {
                    crate::config::save_font_size(self.font_size);
                    self.font_save_pending = None;
                }
            }

            if self.handle_net_messages() {
                redraw_needed = true;
            }

            if redraw_needed {
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }

            let mut deadline = self.next_blink;
            if let Some(t) = self.last_resize {
                deadline = deadline.min(t + RESIZE_DEBOUNCE);
            }
            if let Some(t) = self.font_save_pending {
                deadline = deadline.min(t + FONT_SAVE_DEBOUNCE);
            }
            // Poll network channel roughly every 16ms when connected.
            if self.net_in.is_some() {
                deadline = deadline.min(now + Duration::from_millis(16));
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }

    impl App {
        fn handle_key(&mut self, event_loop: &ActiveEventLoop, ev: KeyEvent) {
            let action = translate(&ev, self.modifiers);
            tracing::debug!(
                ?action,
                logical = ?ev.logical_key,
                cmd = self.modifiers.super_key(),
                ctrl = self.modifiers.control_key(),
                alt = self.modifiers.alt_key(),
                shift = self.modifiers.shift_key(),
                pressed = ev.state.is_pressed(),
                "keyboard event"
            );
            match action {
                KeyAction::CmdEqual => { self.adjust_font_size(FONT_SIZE_STEP); return; }
                KeyAction::CmdMinus => { self.adjust_font_size(-FONT_SIZE_STEP); return; }
                KeyAction::CmdLetter('q') => { event_loop.exit(); return; }
                KeyAction::CmdLetter('c') => {
                    if let Some(text) = self.app_state.editor.selected_text() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
                            Ok(()) => {}
                            Err(e) => tracing::warn!(?e, "clipboard copy failed"),
                        }
                    }
                    return;
                }
                KeyAction::CmdLetter('x') => {
                    if let Some(text) = self.app_state.editor.selected_text() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
                            Ok(()) => {}
                            Err(e) => tracing::warn!(?e, "clipboard cut failed"),
                        }
                        self.app_state.editor.delete_selection();
                        self.reset_blink();
                        self.redraw();
                    }
                    return;
                }
                KeyAction::CmdLetter('v') => {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) if !text.is_empty() => {
                            self.app_state.editor.delete_selection();
                            self.app_state.editor.push_str(&text);
                            self.reset_blink();
                            self.redraw();
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!(?e, "clipboard paste failed"),
                    }
                    return;
                }
                _ => {}
            }

            if !ev.state.is_pressed() {
                return;
            }

            let shift = self.modifiers.shift_key();
            let alt = self.modifiers.alt_key();
            let cmd = self.modifiers.super_key();
            let ctrl = self.modifiers.control_key();

            let changed = match &ev.logical_key {
                Key::Named(NamedKey::Enter) if !cmd && !ctrl => {
                    let text = self.app_state.editor.text().to_string();
                    if !text.is_empty() {
                        self.app_state.conversation.push(RenderedEvent::UserMessage(text.clone()));
                        let queue_mode = if alt { "followup" } else { "steer" };
                        self.send_msg(Msg::Send {
                            workspace_id: 1,
                            text,
                            queue_mode: queue_mode.to_string(),
                        });
                        self.app_state.editor.clear();
                    }
                    true
                }
                Key::Named(NamedKey::Escape) => {
                    if self.app_state.workspace_status == AgentStatusKind::Running {
                        self.send_msg(Msg::Abort { workspace_id: 1 });
                    }
                    false
                }
                Key::Named(NamedKey::ArrowUp) | Key::Named(NamedKey::PageUp) => {
                    if !cmd && !ctrl && !alt {
                        let step = if matches!(&ev.logical_key, Key::Named(NamedKey::PageUp)) {
                            10usize
                        } else {
                            1
                        };
                        self.app_state.scroll_offset =
                            self.app_state.scroll_offset.saturating_add(step);
                        self.app_state.follow_mode = false;
                        true
                    } else {
                        false
                    }
                }
                Key::Named(NamedKey::ArrowDown) | Key::Named(NamedKey::PageDown) => {
                    if !cmd && !ctrl && !alt {
                        let step = if matches!(&ev.logical_key, Key::Named(NamedKey::PageDown)) {
                            10usize
                        } else {
                            1
                        };
                        let new_offset =
                            self.app_state.scroll_offset.saturating_sub(step);
                        self.app_state.scroll_offset = new_offset;
                        if new_offset == 0 {
                            self.app_state.follow_mode = true;
                        }
                        true
                    } else {
                        false
                    }
                }
                Key::Named(NamedKey::Backspace) => {
                    if cmd { self.app_state.editor.delete_to_start(); }
                    else if alt { self.app_state.editor.delete_word_before(); }
                    else { self.app_state.editor.delete_before(); }
                    true
                }
                Key::Named(NamedKey::Delete) => {
                    self.app_state.editor.delete_after();
                    true
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    if cmd { self.app_state.editor.move_to_start(shift); }
                    else if alt { self.app_state.editor.move_word_left(shift); }
                    else { self.app_state.editor.move_left(shift); }
                    true
                }
                Key::Named(NamedKey::ArrowRight) => {
                    if cmd { self.app_state.editor.move_to_end(shift); }
                    else if alt { self.app_state.editor.move_word_right(shift); }
                    else { self.app_state.editor.move_right(shift); }
                    true
                }
                _ if !cmd && !ctrl => {
                    let s = ev.text.as_deref()
                        .filter(|s| !s.is_empty())
                        .or_else(|| match &ev.logical_key {
                            Key::Character(s) => Some(s.as_str()),
                            _ => None,
                        });
                    match s {
                        Some(s) => { self.app_state.editor.push_str(s); true }
                        None => false,
                    }
                }
                _ => false,
            };
            if changed {
                self.reset_blink();
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
        }

        fn adjust_font_size(&mut self, delta: f32) {
            let new_size = (self.font_size + delta).clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
            if (new_size - self.font_size).abs() < 0.01 {
                return;
            }
            self.font_size = new_size;
            self.font_save_pending = Some(Instant::now());
            if let Some(painter) = self.painter.as_mut() {
                painter.set_font_size(new_size);
            }
            self.rebuild_terminal();
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
        }

        fn flush_font_save(&mut self) {
            if self.font_save_pending.take().is_some() {
                crate::config::save_font_size(self.font_size);
            }
        }
    }

    pub(crate) fn run(net: Option<crate::NetChannels>) -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        let mut app = App::new(net);
        event_loop.run_app(&mut app)?;
        Ok(())
    }
}

// =============================================================================
// dev-crossterm: terminal-based ratatui rendering for fast widget iteration
// =============================================================================

#[cfg(feature = "dev-crossterm")]
mod crossterm_main {
    use anyhow::Result;
    use pi_oven_protocol::Msg;
    use pi_oven_ui::{append_agent_event, AgentStatusKind, RenderedEvent};
    use ratatui::backend::CrosstermBackend;
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use ratatui::crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::Terminal;
    use std::io::stdout;
    use std::time::Duration;

    pub(crate) fn run(net: Option<crate::NetChannels>) -> Result<()> {
        let (net_out, net_in) = match net {
            Some(ch) => (Some(ch.out_tx), Some(ch.in_rx)),
            None => (None, None),
        };

        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend)?;

        let mut app_state = pi_oven_ui::AppState::default();

        let send_msg = |msg: Msg| {
            if let Some(tx) = &net_out {
                let _ = tx.blocking_send(msg);
            }
        };

        let result = (|| -> Result<()> {
            loop {
                // Drain incoming network messages.
                if let Some(rx) = &net_in {
                    loop {
                        match rx.try_recv() {
                            Ok(msg) => match msg {
                                Msg::Welcome { workspaces, .. } => {
                                    for ws in &workspaces {
                                        send_msg(Msg::Resume {
                                            workspace_id: ws.workspace_id,
                                            last_seq: app_state.last_seq,
                                        });
                                    }
                                }
                                Msg::ReplayBatch { events, latest_seq, .. } => {
                                    for entry in &events {
                                        append_agent_event(&mut app_state, &entry.event);
                                    }
                                    app_state.last_seq = latest_seq;
                                }
                                Msg::AgentEvent { seq, event, .. } => {
                                    append_agent_event(&mut app_state, &event);
                                    app_state.last_seq = seq;
                                }
                                Msg::AgentStatus { status, .. } => {
                                    app_state.workspace_status = if status == "running" {
                                        AgentStatusKind::Running
                                    } else {
                                        AgentStatusKind::Idle
                                    };
                                }
                                _ => {}
                            },
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                        }
                    }
                }

                terminal.draw(|f| {
                    pi_oven_ui::render(f, &mut app_state);
                })?;

                if event::poll(Duration::from_millis(16))? {
                    if let Event::Key(k) = event::read()? {
                        if k.kind == KeyEventKind::Press {
                            match (k.code, k.modifiers) {
                                (KeyCode::Esc, m)
                                    if !m.contains(KeyModifiers::ALT)
                                        && !m.contains(KeyModifiers::CONTROL) =>
                                {
                                    if app_state.workspace_status == AgentStatusKind::Running {
                                        send_msg(Msg::Abort { workspace_id: 1 });
                                    } else {
                                        break;
                                    }
                                }
                                (KeyCode::Enter, m) if !m.contains(KeyModifiers::CONTROL) => {
                                    let text = app_state.editor.text().to_string();
                                    if !text.is_empty() {
                                        app_state.conversation.push(
                                            RenderedEvent::UserMessage(text.clone()),
                                        );
                                        let queue_mode =
                                            if m.contains(KeyModifiers::ALT) {
                                                "followup"
                                            } else {
                                                "steer"
                                            };
                                        send_msg(Msg::Send {
                                            workspace_id: 1,
                                            text,
                                            queue_mode: queue_mode.to_string(),
                                        });
                                        app_state.editor.clear();
                                    }
                                }
                                (KeyCode::Up, _) => {
                                    app_state.scroll_offset =
                                        app_state.scroll_offset.saturating_add(1);
                                    app_state.follow_mode = false;
                                }
                                (KeyCode::Down, _) => {
                                    let new_offset =
                                        app_state.scroll_offset.saturating_sub(1);
                                    app_state.scroll_offset = new_offset;
                                    if new_offset == 0 {
                                        app_state.follow_mode = true;
                                    }
                                }
                                (KeyCode::PageUp, _) => {
                                    app_state.scroll_offset =
                                        app_state.scroll_offset.saturating_add(10);
                                    app_state.follow_mode = false;
                                }
                                (KeyCode::PageDown, _) => {
                                    let new_offset =
                                        app_state.scroll_offset.saturating_sub(10);
                                    app_state.scroll_offset = new_offset;
                                    if new_offset == 0 {
                                        app_state.follow_mode = true;
                                    }
                                }
                                (KeyCode::Backspace, _) => {
                                    app_state.editor.delete_before();
                                }
                                (KeyCode::Delete, _) => {
                                    app_state.editor.delete_after();
                                }
                                (KeyCode::Left, _) => {
                                    app_state.editor.move_left(false);
                                }
                                (KeyCode::Right, _) => {
                                    app_state.editor.move_right(false);
                                }
                                (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => {
                                    if let Some(text) = app_state.editor.selected_text() {
                                        match arboard::Clipboard::new()
                                            .and_then(|mut cb| cb.set_text(text))
                                        {
                                            Ok(()) => {}
                                            Err(e) => tracing::warn!(?e, "clipboard copy failed"),
                                        }
                                    }
                                }
                                (KeyCode::Char('x'), m) if m.contains(KeyModifiers::CONTROL) => {
                                    if let Some(text) = app_state.editor.selected_text() {
                                        match arboard::Clipboard::new()
                                            .and_then(|mut cb| cb.set_text(text))
                                        {
                                            Ok(()) => {}
                                            Err(e) => tracing::warn!(?e, "clipboard cut failed"),
                                        }
                                        app_state.editor.delete_selection();
                                    }
                                }
                                (KeyCode::Char('v'), m) if m.contains(KeyModifiers::CONTROL) => {
                                    match arboard::Clipboard::new()
                                        .and_then(|mut cb| cb.get_text())
                                    {
                                        Ok(text) if !text.is_empty() => {
                                            app_state.editor.delete_selection();
                                            app_state.editor.push_str(&text);
                                        }
                                        Ok(_) => {}
                                        Err(e) => tracing::warn!(?e, "clipboard paste failed"),
                                    }
                                }
                                (KeyCode::Char(c), _) => {
                                    let mut buf = [0u8; 4];
                                    app_state.editor.push_str(c.encode_utf8(&mut buf));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Ok(())
        })();

        let mut out = std::io::stdout();
        execute!(out, LeaveAlternateScreen).ok();
        disable_raw_mode().ok();
        result
    }
}
