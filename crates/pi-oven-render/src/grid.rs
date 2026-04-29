//! In-memory cell grid: the canonical data the renderer paints each frame.
//!
//! The grid stores one [`Cell`] per `(x, y)` position, where `x` is the column
//! and `y` is the row. Colours and attributes are kept as ratatui's own
//! [`Color`] and [`Modifier`] types so styling is preserved verbatim from the
//! widgets that wrote them; the paint pipeline (in `paint.rs`) is responsible
//! for resolving them to RGB at draw time.

use ratatui::style::{Color, Modifier};

/// Re-export so callers can write `pi_oven_render::Attrs`.
pub type Attrs = Modifier;

/// One cell in the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            fg: Color::Reset,
            bg: Color::Reset,
            attrs: Modifier::empty(),
        }
    }
}

/// Rectangular grid of [`Cell`]s, addressed `(x, y)` with `x` along the row.
#[derive(Debug, Clone)]
pub struct Grid {
    cols: u16,
    rows: u16,
    cells: Vec<Cell>,
}

impl Grid {
    /// Create a fresh grid filled with [`Cell::default`].
    pub fn new(cols: u16, rows: u16) -> Self {
        let len = cols as usize * rows as usize;
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); len],
        }
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Resize and clear the grid. Existing contents are discarded — callers
    /// that want to preserve text across a resize must redraw.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.cells.clear();
        self.cells
            .resize(cols as usize * rows as usize, Cell::default());
    }

    /// Write `cell` into position `(x, y)`. Out-of-range coordinates are
    /// silently ignored to match ratatui's clipping behaviour.
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x >= self.cols || y >= self.rows {
            return;
        }
        let idx = y as usize * self.cols as usize + x as usize;
        self.cells[idx] = cell;
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        if x >= self.cols || y >= self.rows {
            return None;
        }
        let idx = y as usize * self.cols as usize + x as usize;
        self.cells.get(idx)
    }

    pub fn fill(&mut self, cell: Cell) {
        self.cells.iter_mut().for_each(|c| *c = cell);
    }
}
