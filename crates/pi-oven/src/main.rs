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

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}

#[cfg(feature = "dev-wgpu")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-wgpu)");
    wgpu_main::run()
}

#[cfg(feature = "dev-crossterm")]
fn main() -> anyhow::Result<()> {
    init_tracing();
    tracing::info!("pi-oven starting (dev-crossterm)");
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
    use ratatui::layout::Rect;
    use ratatui::widgets::Paragraph;
    use ratatui::Terminal;
    use winit::application::ApplicationHandler;
    use winit::event::{KeyEvent, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::keyboard::ModifiersState;
    use winit::window::{Window, WindowAttributes, WindowId};

    use crate::keys::{translate, KeyAction};

    const FONT_SIZE_PX: f32 = 14.0;

    struct App {
        window: Option<Arc<Window>>,
        painter: Option<Painter>,
        terminal: Option<Terminal<RatatuiGridBackend>>,
        modifiers: ModifiersState,
    }

    impl App {
        fn new() -> Self {
            Self {
                window: None,
                painter: None,
                terminal: None,
                modifiers: ModifiersState::empty(),
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
                let area = Rect::new(0, 0, f.area().width, 1);
                f.render_widget(Paragraph::new("pi-oven"), area);
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
                alt = self.modifiers.alt_key(),
                shift = self.modifiers.shift_key(),
                pressed = ev.state.is_pressed(),
                "keyboard event"
            );
            if matches!(action, KeyAction::CmdW) {
                event_loop.exit();
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
    use ratatui::layout::Rect;
    use ratatui::widgets::Paragraph;
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
                    let area = Rect::new(0, 0, f.area().width, 1);
                    f.render_widget(Paragraph::new("pi-oven"), area);
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
