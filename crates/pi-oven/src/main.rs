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

const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

/// Spawn the tokio runtime on a dedicated OS thread and start the reconnecting
/// WebSocket client.  Communicates with the winit/crossterm thread via the
/// returned channel receiver.
///
/// If `PI_OVEN_SHARED_KEY` is absent, logs a warning and returns `None` —
/// the UI continues to work for development without networking (task 9.1).
fn spawn_network(
    net_event_tx: std::sync::mpsc::Sender<pi_oven_net::ConnectionState>,
) -> Option<()> {
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

    // Run the tokio runtime on a separate thread so it doesn't interfere with
    // the winit event loop on the main thread (task 9.2).
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            let handle = pi_oven_net::start_reconnecting(url, key, CLIENT_VERSION.to_string());
            let mut state_rx = handle.state_rx;

            loop {
                let state = state_rx.borrow().clone();

                // Log state changes at info level (task 9.3).
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

                // Forward state to the UI thread.
                if net_event_tx.send(state).is_err() {
                    break; // UI thread gone.
                }

                if state_rx.changed().await.is_err() {
                    break; // Reconnect task ended.
                }
            }
        });
    });

    Some(())
}

#[cfg(feature = "dev-wgpu")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-wgpu)");
    let (net_tx, _net_rx) = std::sync::mpsc::channel::<pi_oven_net::ConnectionState>();
    spawn_network(net_tx);
    wgpu_main::run()
}

#[cfg(feature = "dev-crossterm")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-crossterm)");
    let (net_tx, _net_rx) = std::sync::mpsc::channel::<pi_oven_net::ConnectionState>();
    spawn_network(net_tx);
    crossterm_main::run()
}

// =============================================================================
// dev-wgpu: native winit window + wgpu/glyphon paint via pi-oven-render
// =============================================================================

#[cfg(feature = "dev-wgpu")]
mod wgpu_main {
    use std::sync::Arc;

    use anyhow::Result;
    use pi_oven_render::{Painter, RatatuiGridBackend};
    use ratatui::Terminal;
    use winit::application::ApplicationHandler;
    use winit::event::{KeyEvent, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::keyboard::ModifiersState;
    use winit::window::{Window, WindowAttributes, WindowId};

    use crate::keys::{translate, KeyAction};

    const FONT_SIZE_PX: f32 = 18.0;
    const FONT_SIZE_STEP: f32 = 2.0;
    const FONT_SIZE_MIN: f32 = 8.0;
    const FONT_SIZE_MAX: f32 = 48.0;

    struct App {
        window: Option<Arc<Window>>,
        painter: Option<Painter>,
        terminal: Option<Terminal<RatatuiGridBackend>>,
        modifiers: ModifiersState,
        font_size: f32,
    }

    impl App {
        fn new() -> Self {
            Self {
                window: None,
                painter: None,
                terminal: None,
                modifiers: ModifiersState::empty(),
                font_size: FONT_SIZE_PX,
            }
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
            if let Err(e) = terminal.draw(|f| {
                pi_oven_ui::render(f);
            }) {
                tracing::error!(?e, "ratatui draw failed");
                return;
            }
            if let Err(e) = painter.paint(terminal.backend().grid()) {
                tracing::error!(?e, "wgpu paint failed");
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

            match pollster::block_on(Painter::new(window.clone(), FONT_SIZE_PX)) {
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
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    if let Some(p) = self.painter.as_mut() {
                        p.resize(size);
                        self.rebuild_terminal();
                    }
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
                KeyAction::CmdW => event_loop.exit(),
                KeyAction::CmdEqual => self.adjust_font_size(FONT_SIZE_STEP),
                KeyAction::CmdMinus => self.adjust_font_size(-FONT_SIZE_STEP),
                _ => {}
            }
        }

        fn adjust_font_size(&mut self, delta: f32) {
            let new_size = (self.font_size + delta).clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
            if (new_size - self.font_size).abs() < 0.01 {
                return;
            }
            self.font_size = new_size;
            if let Some(painter) = self.painter.as_mut() {
                painter.set_font_size(new_size);
            }
            self.rebuild_terminal();
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
        }
    }

    pub(crate) fn run() -> Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        let mut app = App::new();
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
    use ratatui::backend::CrosstermBackend;
    use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::Terminal;
    use std::io::stdout;
    use std::time::Duration;

    pub(crate) fn run() -> Result<()> {
        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(out);
        let mut terminal = Terminal::new(backend)?;

        let result = (|| -> Result<()> {
            loop {
                terminal.draw(|f| {
                    pi_oven_ui::render(f);
                })?;

                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(k) = event::read()? {
                        if k.kind == KeyEventKind::Press {
                            match k.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
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
