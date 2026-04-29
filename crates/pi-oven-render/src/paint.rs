//! wgpu + glyphon paint pipeline.
//!
//! The painter owns the GPU resources tied to a single window: a wgpu
//! `Surface`, `Device`, `Queue`, plus glyphon's `FontSystem`, `SwashCache`,
//! `TextAtlas`, and `TextRenderer`. Each frame, [`Painter::paint`] reads the
//! current [`Grid`] and produces a glyphon `Buffer` whose contents are one
//! styled run per contiguous same-style cell range; that buffer is then
//! rendered into the surface, after clearing to the configured background
//! colour first.
//!
//! API drift note: wgpu, glyphon and winit pin tightly to each other and
//! their public APIs change between minor versions. If `cargo build` reports
//! mismatches, prefer adjusting the calls below over downgrading pins —
//! the surrounding shape (resource ownership, paint flow, resize handling)
//! is what's load-bearing.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glyphon::{
    Attrs, Buffer, Cache, Color as GColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use ratatui::style::{Color as RColor, Modifier};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::grid::{Cell, Grid};

/// 0–255 RGBA. Stored as four `u8`s rather than [f64; 4] so the conversion to
/// wgpu's `Color` is trivial and lossless.
#[derive(Debug, Clone, Copy)]
pub struct ClearColor(pub [u8; 4]);

impl Default for ClearColor {
    fn default() -> Self {
        Self([0x12, 0x12, 0x12, 0xff])
    }
}

impl From<ClearColor> for wgpu::Color {
    fn from(c: ClearColor) -> Self {
        let [r, g, b, a] = c.0;
        wgpu::Color {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: a as f64 / 255.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    pub cell_width_px: f32,
    pub line_height_px: f32,
    pub font_size_px: f32,
}

pub struct Painter {
    #[allow(dead_code)]
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    font_system: FontSystem,
    swash_cache: SwashCache,
    #[allow(dead_code)]
    cache: Cache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,

    metrics: CellMetrics,
    clear_color: ClearColor,
    #[allow(dead_code)]
    scale_factor: f64,
}

impl Painter {
    /// Construct the painter for `window`. Loads any `.ttf` files under
    /// `crates/pi-oven-render/assets/fonts/` into glyphon's `FontSystem`; if
    /// none are present the OS's system fonts are used.
    pub async fn new(window: Arc<Window>, font_size_px: f32) -> Result<Self> {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .context("create wgpu surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("no wgpu adapter available"))?;

        // Use the adapter's reported limits rather than the conservative
        // downlevel defaults — the latter cap textures at 2048×2048, which
        // is smaller than a single retina-display surface.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("pi-oven device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter.limits(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .context("request device")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps
                .present_modes
                .iter()
                .copied()
                .find(|m| matches!(m, wgpu::PresentMode::Mailbox))
                .unwrap_or(wgpu::PresentMode::Fifo),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let mut font_system = FontSystem::new();
        load_bundled_fonts(&mut font_system)?;

        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);

        // Size a monospace cell as `font_size * 0.6` wide × `font_size * 1.25`
        // tall — close enough for a uniform monospace family. The painter
        // does not measure glyph extents itself; widget code that cares about
        // exact layout should query [`Painter::cell_metrics`].
        let metrics = CellMetrics {
            cell_width_px: font_size_px * 0.6,
            line_height_px: font_size_px * 1.25,
            font_size_px,
        };

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            font_system,
            swash_cache,
            cache,
            viewport,
            atlas,
            text_renderer,
            metrics,
            clear_color: ClearColor::default(),
            scale_factor,
        })
    }

    pub fn cell_metrics(&self) -> CellMetrics {
        self.metrics
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.surface_config.width, self.surface_config.height)
    }

    /// Returns the grid dimensions (cols, rows) the current surface size + cell
    /// metrics imply. Callers should pass the result back into
    /// [`crate::Grid::resize`] when reacting to a resize event.
    pub fn grid_dimensions(&self) -> (u16, u16) {
        let cols = (self.surface_config.width as f32 / self.metrics.cell_width_px).floor() as u16;
        let rows = (self.surface_config.height as f32 / self.metrics.line_height_px).floor() as u16;
        (cols.max(1), rows.max(1))
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn scale_factor_changed(&mut self, new_scale: f64) {
        self.scale_factor = new_scale;
        // The window will deliver a follow-up `Resized` event with the new
        // physical size; we react there.
    }

    /// Render `grid` to the surface. Clears to the configured background
    /// colour, then draws one styled glyphon run per contiguous same-style
    /// cell range.
    pub fn paint(&mut self, grid: &Grid) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            Err(e) => return Err(anyhow!("surface error: {e:?}")),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("pi-oven encoder"),
                });

        // Build the glyphon Buffer from the grid: one Attrs span per
        // contiguous run of cells with identical (fg, bg, attrs).
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(self.metrics.font_size_px, self.metrics.line_height_px),
        );
        buffer.set_size(
            &mut self.font_system,
            Some(self.surface_config.width as f32),
            Some(self.surface_config.height as f32),
        );
        buffer.set_wrap(&mut self.font_system, Wrap::None);

        let lines = build_lines(grid);
        let spans: Vec<(&str, Attrs)> = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| (s.text.as_str(), s.attrs.clone())))
            .collect();
        let default_attrs = Attrs::new().family(Family::Monospace);
        buffer.set_rich_text(
            &mut self.font_system,
            spans.iter().map(|(t, a)| (*t, a.clone())),
            default_attrs,
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [TextArea {
                    buffer: &buffer,
                    left: 0.0,
                    top: 0.0,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: self.surface_config.width as i32,
                        bottom: self.surface_config.height as i32,
                    },
                    default_color: GColor::rgb(0xff, 0xff, 0xff),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .context("prepare glyphon text")?;

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pi-oven main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)
                .context("render glyphon text")?;
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(())
    }
}

/// One styled span within a row.
struct Span {
    text: String,
    attrs: Attrs<'static>,
}

/// One row's worth of styled spans, terminated by an implicit '\n'.
struct Line {
    spans: Vec<Span>,
}

fn build_lines(grid: &Grid) -> Vec<Line> {
    let mut lines = Vec::with_capacity(grid.rows() as usize);
    for y in 0..grid.rows() {
        let mut spans: Vec<Span> = Vec::new();
        let mut current: Option<(RColor, RColor, Modifier, String)> = None;

        for x in 0..grid.cols() {
            let cell = grid.get(x, y).copied().unwrap_or(Cell::default());
            match &mut current {
                Some(state) if state.0 == cell.fg && state.1 == cell.bg && state.2 == cell.attrs => {
                    state.3.push(cell.ch);
                }
                _ => {
                    if let Some(state) = current.take() {
                        spans.push(state_to_span(state));
                    }
                    let mut text = String::new();
                    text.push(cell.ch);
                    current = Some((cell.fg, cell.bg, cell.attrs, text));
                }
            }
        }
        if let Some(state) = current.take() {
            spans.push(state_to_span(state));
        }
        // Implicit row separator.
        if y + 1 < grid.rows() {
            spans.push(Span {
                text: "\n".to_string(),
                attrs: Attrs::new().family(Family::Monospace),
            });
        }
        lines.push(Line { spans });
    }
    lines
}

fn state_to_span(state: (RColor, RColor, Modifier, String)) -> Span {
    let (fg, _bg, modifiers, text) = state;
    let mut attrs = Attrs::new().family(Family::Monospace);
    if let Some(c) = ratatui_color_to_glyphon(fg) {
        attrs = attrs.color(c);
    }
    if modifiers.contains(Modifier::BOLD) {
        attrs = attrs.weight(glyphon::Weight::BOLD);
    }
    if modifiers.contains(Modifier::ITALIC) {
        attrs = attrs.style(glyphon::Style::Italic);
    }
    // glyphon does not directly model BG; per-cell backgrounds are deferred
    // to a future change that draws coloured quads beneath the text.
    Span { text, attrs }
}

fn ratatui_color_to_glyphon(c: RColor) -> Option<GColor> {
    match c {
        RColor::Reset => None,
        RColor::Black => Some(GColor::rgb(0, 0, 0)),
        RColor::Red => Some(GColor::rgb(0xcc, 0x33, 0x33)),
        RColor::Green => Some(GColor::rgb(0x33, 0xcc, 0x33)),
        RColor::Yellow => Some(GColor::rgb(0xcc, 0xcc, 0x33)),
        RColor::Blue => Some(GColor::rgb(0x33, 0x33, 0xcc)),
        RColor::Magenta => Some(GColor::rgb(0xcc, 0x33, 0xcc)),
        RColor::Cyan => Some(GColor::rgb(0x33, 0xcc, 0xcc)),
        RColor::Gray => Some(GColor::rgb(0xaa, 0xaa, 0xaa)),
        RColor::DarkGray => Some(GColor::rgb(0x55, 0x55, 0x55)),
        RColor::LightRed => Some(GColor::rgb(0xff, 0x66, 0x66)),
        RColor::LightGreen => Some(GColor::rgb(0x66, 0xff, 0x66)),
        RColor::LightYellow => Some(GColor::rgb(0xff, 0xff, 0x66)),
        RColor::LightBlue => Some(GColor::rgb(0x66, 0x66, 0xff)),
        RColor::LightMagenta => Some(GColor::rgb(0xff, 0x66, 0xff)),
        RColor::LightCyan => Some(GColor::rgb(0x66, 0xff, 0xff)),
        RColor::White => Some(GColor::rgb(0xff, 0xff, 0xff)),
        RColor::Rgb(r, g, b) => Some(GColor::rgb(r, g, b)),
        RColor::Indexed(_) => None,
    }
}

fn load_bundled_fonts(font_system: &mut FontSystem) -> Result<()> {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let read = match std::fs::read_dir(&assets_dir) {
        Ok(r) => r,
        Err(_) => return Ok(()), // No assets dir is fine; fall back to system fonts.
    };
    let db = font_system.db_mut();
    for entry in read.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("ttf") || e.eq_ignore_ascii_case("otf"))
            .unwrap_or(false)
        {
            db.load_font_file(&path)
                .map_err(|e| anyhow!("load font {}: {e:?}", path.display()))?;
        }
    }
    Ok(())
}
