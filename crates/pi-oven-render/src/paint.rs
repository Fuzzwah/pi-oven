//! wgpu + glyphon paint pipeline.
//!
//! The painter owns the GPU resources tied to a single window: a wgpu
//! `Surface`, `Device`, `Queue`, plus glyphon's `FontSystem`, `SwashCache`,
//! `TextAtlas`, and `TextRenderer`.
//!
//! Each frame, [`Painter::paint`] receives the set of grid rows that changed
//! since the last call. Only those rows are re-shaped; the rest reuse their
//! cached per-row [`Buffer`]s. This keeps the hot path O(changed_rows) rather
//! than O(all_cells), making keystroke rendering essentially free.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use glyphon::{
    Attrs, Buffer, Cache, Color as GColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap,
};
use ratatui::style::{Color as RColor, Modifier};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::grid::{Cell, Grid};

const RECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;
const UNDERLINE_HEIGHT_RATIO: f32 = 0.08;
const UNDERLINE_OFFSET_RATIO: f32 = 0.12;

/// 0–255 RGBA.
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
    rect_pipeline: wgpu::RenderPipeline,

    metrics: CellMetrics,
    clear_color: ClearColor,
    #[allow(dead_code)]
    scale_factor: f64,

    /// One shaped glyphon Buffer per grid row. Rebuilt only for dirty rows;
    /// unchanged rows keep their cached Buffer across frames.
    row_buffers: Vec<Buffer>,

    rect_gpu_buf: Option<wgpu::Buffer>,
    rect_vertex_count: u32,
}

impl Painter {
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
            .or_else(|| surface_caps.formats.first().copied())
            .ok_or_else(|| anyhow!("surface has no supported formats"))?;
        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| anyhow!("surface has no supported alpha modes"))?;

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
            alpha_mode,
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
        let rect_pipeline = create_rect_pipeline(&device, surface_format);

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
            rect_pipeline,
            metrics,
            clear_color: ClearColor::default(),
            scale_factor,
            row_buffers: Vec::new(),
            rect_gpu_buf: None,
            rect_vertex_count: 0,
        })
    }

    pub fn cell_metrics(&self) -> CellMetrics {
        self.metrics
    }

    pub fn set_font_size(&mut self, font_size_px: f32) {
        self.metrics = CellMetrics {
            cell_width_px: font_size_px * 0.6,
            line_height_px: font_size_px * 1.25,
            font_size_px,
        };
        // Discard cached row buffers; rebuild_terminal() creates a fresh
        // backend that marks all rows dirty on the next frame.
        self.row_buffers.clear();
        self.rect_gpu_buf = None;
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.surface_config.width, self.surface_config.height)
    }

    pub fn grid_dimensions(&self) -> (u16, u16) {
        let cols = (self.surface_config.width as f32 / self.metrics.cell_width_px).floor() as u16;
        let rows =
            (self.surface_config.height as f32 / self.metrics.line_height_px).floor() as u16;
        (cols.max(1), rows.max(1))
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        // Row buffers are invalidated implicitly: rebuild_terminal() creates a
        // fresh backend that marks every row dirty on the first draw.
        self.row_buffers.clear();
        self.rect_gpu_buf = None;
    }

    pub fn scale_factor_changed(&mut self, new_scale: f64) {
        self.scale_factor = new_scale;
    }

    /// Render `grid` to the surface.
    ///
    /// `dirty_rows` contains the row indices that changed since the last call
    /// (from [`RatatuiGridBackend::take_dirty_rows`]). Only those rows are
    /// re-shaped; every other row reuses its cached glyphon [`Buffer`].
    /// An empty slice means nothing changed — the painter reuses all cached
    /// GPU buffers and only pays for the GPU render pass itself.
    pub fn paint(&mut self, grid: &Grid, dirty_rows: &[u16]) -> Result<()> {
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

        // --- Sync row_buffers length to current grid height ---------------------
        let grid_rows = grid.rows() as usize;
        let font_size = self.metrics.font_size_px;
        let line_height = self.metrics.line_height_px;
        let surf_width = self.surface_config.width as f32;

        while self.row_buffers.len() < grid_rows {
            let mut buf =
                Buffer::new(&mut self.font_system, Metrics::new(font_size, line_height));
            buf.set_size(&mut self.font_system, Some(surf_width), Some(line_height));
            buf.set_wrap(&mut self.font_system, Wrap::None);
            self.row_buffers.push(buf);
        }
        self.row_buffers.truncate(grid_rows);

        // --- Reshape only the rows that changed ---------------------------------
        for &y in dirty_rows {
            let y_idx = y as usize;
            if y_idx >= self.row_buffers.len() {
                continue;
            }
            let buf = &mut self.row_buffers[y_idx];
            buf.set_metrics(&mut self.font_system, Metrics::new(font_size, line_height));
            let spans = build_row_spans(grid, y);
            let default_attrs = Attrs::new().family(Family::Monospace);
            buf.set_rich_text(
                &mut self.font_system,
                spans.iter().map(|(t, a)| (t.as_str(), a.clone())),
                default_attrs,
                Shaping::Basic,
            );
            buf.shape_until_scroll(&mut self.font_system, false);
        }

        // --- Rebuild fill-rect GPU buffer when anything changed -----------------
        if !dirty_rows.is_empty() || self.rect_gpu_buf.is_none() {
            let fill_rects = build_fill_rects(grid, self.metrics);
            let rect_vertices = fill_rects_to_vertices(
                &fill_rects,
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            );
            self.rect_vertex_count = rect_vertices.len() as u32;
            self.rect_gpu_buf = (!rect_vertices.is_empty()).then(|| {
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("pi-oven fill rect buffer"),
                        contents: bytemuck::cast_slice(&rect_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    })
            });
        }

        // --- Prepare glyphon (atlas lookup + upload) ----------------------------
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.surface_config.width,
                height: self.surface_config.height,
            },
        );

        {
            // Build TextAreas borrowing from row_buffers; block ends before
            // the render pass so the immutable borrow is released cleanly.
            let text_areas: Vec<TextArea<'_>> = self
                .row_buffers
                .iter()
                .enumerate()
                .map(|(y, buf)| TextArea {
                    buffer: buf,
                    left: 0.0,
                    top: y as f32 * line_height,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: self.surface_config.width as i32,
                        bottom: self.surface_config.height as i32,
                    },
                    default_color: GColor::rgb(0xff, 0xff, 0xff),
                    custom_glyphs: &[],
                })
                .collect();

            self.text_renderer
                .prepare(
                    &self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    text_areas,
                    &mut self.swash_cache,
                )
                .context("prepare glyphon text")?;
        }

        // --- Render pass --------------------------------------------------------
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
            if let Some(rect_buf) = self.rect_gpu_buf.as_ref() {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_vertex_buffer(0, rect_buf.slice(..));
                pass.draw(0..self.rect_vertex_count, 0..1);
            }
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

// =============================================================================
// Row span building — hot path, called only for dirty rows
// =============================================================================

/// Build styled spans for a single grid row. Returns `Vec<(text, Attrs)>`
/// suitable for `Buffer::set_rich_text`.
fn build_row_spans(grid: &Grid, y: u16) -> Vec<(String, Attrs<'static>)> {
    let mut spans: Vec<(String, Attrs<'static>)> = Vec::new();
    let mut current: Option<(RColor, RColor, Modifier, String)> = None;

    for x in 0..grid.cols() {
        let cell = grid.get(x, y).cloned().unwrap_or_default();
        match &mut current {
            Some(state)
                if state.0 == cell.fg && state.1 == cell.bg && state.2 == cell.attrs =>
            {
                state.3.push_str(&cell.symbol);
            }
            _ => {
                if let Some(state) = current.take() {
                    spans.push(state_to_attrs(state));
                }
                current = Some((cell.fg, cell.bg, cell.attrs, cell.symbol.clone()));
            }
        }
    }
    if let Some(state) = current.take() {
        spans.push(state_to_attrs(state));
    }
    spans
}

fn state_to_attrs(state: (RColor, RColor, Modifier, String)) -> (String, Attrs<'static>) {
    let (fg, bg, modifiers, text) = state;
    let mut attrs = Attrs::new().family(Family::Monospace);
    // REVERSED swaps fg/bg; for glyphs we care about the effective foreground.
    let effective_fg = if modifiers.contains(Modifier::REVERSED) { bg } else { fg };
    match ratatui_color_to_glyphon(effective_fg) {
        Some(c) => { attrs = attrs.color(c); }
        None if modifiers.contains(Modifier::REVERSED) => {
            // bg was Reset → effective fg should match our dark background
            attrs = attrs.color(GColor::rgb(0x12, 0x12, 0x12));
        }
        None => {} // fg was Reset → fall through to TextArea default_color (white)
    }
    if modifiers.contains(Modifier::BOLD) {
        attrs = attrs.weight(glyphon::Weight::BOLD);
    }
    if modifiers.contains(Modifier::ITALIC) {
        attrs = attrs.style(glyphon::Style::Italic);
    }
    (text, attrs)
}

// =============================================================================
// Fill-rect geometry — background colors + underlines
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
struct FillRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FillVertex {
    position: [f32; 2],
    color: [f32; 4],
}

fn build_fill_rects(grid: &Grid, metrics: CellMetrics) -> Vec<FillRect> {
    let mut rects = Vec::new();
    let underline_height = (metrics.font_size_px * UNDERLINE_HEIGHT_RATIO).max(1.0);
    let underline_offset = (metrics.line_height_px * UNDERLINE_OFFSET_RATIO).max(1.0);

    for y in 0..grid.rows() {
        for (start, end, color) in collect_row_runs(grid, y, |cell| {
            if cell.attrs.contains(Modifier::REVERSED) {
                // REVERSED: use fg as the background color; Reset fg → white
                Some(match cell.fg {
                    RColor::Reset => [1.0, 1.0, 1.0, 1.0],
                    c => ratatui_color_to_rgba(c).unwrap_or([1.0, 1.0, 1.0, 1.0]),
                })
            } else {
                ratatui_color_to_rgba(cell.bg)
            }
        }) {
            rects.push(FillRect {
                left: start * metrics.cell_width_px,
                top: y as f32 * metrics.line_height_px,
                right: end * metrics.cell_width_px,
                bottom: (y as f32 + 1.0) * metrics.line_height_px,
                color,
            });
        }
        for (start, end, color) in collect_row_runs(grid, y, |cell| {
            cell.attrs
                .contains(Modifier::UNDERLINED)
                .then(|| ratatui_color_to_rgba(cell.fg).unwrap_or([1.0, 1.0, 1.0, 1.0]))
        }) {
            let top = (y as f32 + 1.0) * metrics.line_height_px
                - underline_offset
                - underline_height;
            rects.push(FillRect {
                left: start * metrics.cell_width_px,
                top,
                right: end * metrics.cell_width_px,
                bottom: top + underline_height,
                color,
            });
        }
    }
    rects
}

fn collect_row_runs<F>(grid: &Grid, y: u16, color_for_cell: F) -> Vec<(f32, f32, [f32; 4])>
where
    F: Fn(&Cell) -> Option<[f32; 4]>,
{
    let mut runs = Vec::new();
    let mut run_start: Option<u16> = None;
    let mut run_color: Option<[f32; 4]> = None;

    for x in 0..grid.cols() {
        let cell = grid.get(x, y).cloned().unwrap_or_default();
        let cell_color = color_for_cell(&cell);
        if cell_color.is_some() && cell_color == run_color {
            continue;
        }
        if let (Some(start), Some(color)) = (run_start.take(), run_color.take()) {
            runs.push((start as f32, x as f32, color));
        }
        if let Some(color) = cell_color {
            run_start = Some(x);
            run_color = Some(color);
        }
    }
    if let (Some(start), Some(color)) = (run_start.take(), run_color.take()) {
        runs.push((start as f32, grid.cols() as f32, color));
    }
    runs
}

fn fill_rects_to_vertices(rects: &[FillRect], width: f32, height: f32) -> Vec<FillVertex> {
    let mut vertices = Vec::with_capacity(rects.len() * 6);
    for rect in rects {
        let left = px_to_ndc_x(rect.left, width);
        let right = px_to_ndc_x(rect.right, width);
        let top = px_to_ndc_y(rect.top, height);
        let bottom = px_to_ndc_y(rect.bottom, height);
        let color = rect.color;
        vertices.extend_from_slice(&[
            FillVertex { position: [left, top], color },
            FillVertex { position: [right, top], color },
            FillVertex { position: [right, bottom], color },
            FillVertex { position: [left, top], color },
            FillVertex { position: [right, bottom], color },
            FillVertex { position: [left, bottom], color },
        ]);
    }
    vertices
}

fn px_to_ndc_x(x: f32, width: f32) -> f32 {
    (x / width) * 2.0 - 1.0
}

fn px_to_ndc_y(y: f32, height: f32) -> f32 {
    1.0 - (y / height) * 2.0
}

// =============================================================================
// Color utilities
// =============================================================================

fn ratatui_color_to_glyphon(c: RColor) -> Option<GColor> {
    ratatui_color_to_rgba_u8(c).map(|[r, g, b, _]| GColor::rgba(r, g, b, 0xff))
}

fn ratatui_color_to_rgba(c: RColor) -> Option<[f32; 4]> {
    ratatui_color_to_rgba_u8(c).map(|[r, g, b, a]| {
        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    })
}

fn ratatui_color_to_rgba_u8(c: RColor) -> Option<[u8; 4]> {
    let rgb = match c {
        RColor::Reset => return None,
        RColor::Black => [0x00, 0x00, 0x00],
        RColor::Red => [0xcc, 0x33, 0x33],
        RColor::Green => [0x33, 0xcc, 0x33],
        RColor::Yellow => [0xcc, 0xcc, 0x33],
        RColor::Blue => [0x33, 0x33, 0xcc],
        RColor::Magenta => [0xcc, 0x33, 0xcc],
        RColor::Cyan => [0x33, 0xcc, 0xcc],
        RColor::Gray => [0xaa, 0xaa, 0xaa],
        RColor::DarkGray => [0x55, 0x55, 0x55],
        RColor::LightRed => [0xff, 0x66, 0x66],
        RColor::LightGreen => [0x66, 0xff, 0x66],
        RColor::LightYellow => [0xff, 0xff, 0x66],
        RColor::LightBlue => [0x66, 0x66, 0xff],
        RColor::LightMagenta => [0xff, 0x66, 0xff],
        RColor::LightCyan => [0x66, 0xff, 0xff],
        RColor::White => [0xff, 0xff, 0xff],
        RColor::Rgb(r, g, b) => [r, g, b],
        RColor::Indexed(i) => ansi_index_to_rgb(i),
    };
    Some([rgb[0], rgb[1], rgb[2], 0xff])
}

fn ansi_index_to_rgb(index: u8) -> [u8; 3] {
    const ANSI_16: [[u8; 3]; 16] = [
        [0x00, 0x00, 0x00],
        [0x80, 0x00, 0x00],
        [0x00, 0x80, 0x00],
        [0x80, 0x80, 0x00],
        [0x00, 0x00, 0x80],
        [0x80, 0x00, 0x80],
        [0x00, 0x80, 0x80],
        [0xc0, 0xc0, 0xc0],
        [0x80, 0x80, 0x80],
        [0xff, 0x00, 0x00],
        [0x00, 0xff, 0x00],
        [0xff, 0xff, 0x00],
        [0x00, 0x00, 0xff],
        [0xff, 0x00, 0xff],
        [0x00, 0xff, 0xff],
        [0xff, 0xff, 0xff],
    ];
    match index {
        0..=15 => ANSI_16[index as usize],
        16..=231 => {
            let idx = index - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            [cube_channel(r), cube_channel(g), cube_channel(b)]
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            [gray, gray, gray]
        }
    }
}

fn cube_channel(component: u8) -> u8 {
    if component == 0 { 0 } else { 55 + component * 40 }
}

// =============================================================================
// Pipeline helpers
// =============================================================================

fn create_rect_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pi-oven rect shader"),
        source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pi-oven rect pipeline layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pi-oven rect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<FillVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    wgpu::VertexAttribute {
                        offset: std::mem::size_of::<[f32; 2]>() as u64,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x4,
                    },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

fn load_bundled_fonts(font_system: &mut FontSystem) -> Result<()> {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let read = match std::fs::read_dir(&assets_dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
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

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::{ansi_index_to_rgb, build_fill_rects, CellMetrics};
    use crate::grid::{Cell, Grid};
    use ratatui::style::{Color, Modifier};

    #[test]
    fn fill_rects_include_background_and_underline_runs() {
        let mut grid = Grid::new(3, 1);
        grid.set(
            0,
            0,
            Cell {
                symbol: "A".into(),
                fg: Color::White,
                bg: Color::Blue,
                attrs: Modifier::empty(),
            },
        );
        grid.set(
            1,
            0,
            Cell {
                symbol: "B".into(),
                fg: Color::Red,
                bg: Color::Blue,
                attrs: Modifier::UNDERLINED,
            },
        );
        grid.set(
            2,
            0,
            Cell {
                symbol: "C".into(),
                fg: Color::Red,
                bg: Color::Reset,
                attrs: Modifier::UNDERLINED,
            },
        );

        let rects = build_fill_rects(
            &grid,
            CellMetrics {
                cell_width_px: 10.0,
                line_height_px: 20.0,
                font_size_px: 16.0,
            },
        );

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].left, 0.0);
        assert_eq!(rects[0].right, 20.0);
        assert_eq!(rects[1].left, 10.0);
        assert_eq!(rects[1].right, 30.0);
        assert!(rects[1].top >= 0.0);
        assert!(rects[1].bottom <= 20.0);
    }

    #[test]
    fn indexed_palette_colors_are_mapped() {
        assert_eq!(ansi_index_to_rgb(9), [0xff, 0x00, 0x00]);
        assert_eq!(ansi_index_to_rgb(46), [0x00, 0xff, 0x00]);
        assert_eq!(ansi_index_to_rgb(244), [0x80, 0x80, 0x80]);
    }
}
