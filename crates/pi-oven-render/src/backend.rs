//! Custom `ratatui::backend::Backend` impl that writes its diff into our
//! [`Grid`](crate::grid::Grid) instead of emitting terminal escape sequences.
//! Widgets remain backend-agnostic; the paint pipeline reads the grid each
//! frame and uploads it to the GPU.

use std::io;

use ratatui::backend::{Backend, WindowSize};
use ratatui::buffer::Cell as RatatuiCell;
use ratatui::layout::{Position, Size};

use crate::grid::{Cell, Grid};

/// Owns a [`Grid`] and forwards ratatui's draw diffs into it.
#[derive(Debug)]
pub struct RatatuiGridBackend {
    grid: Grid,
    cursor: Position,
    cursor_visible: bool,
    /// Rows that received at least one changed cell since the last
    /// [`take_dirty_rows`](Self::take_dirty_rows) call.
    dirty_rows: Vec<u16>,
}

impl RatatuiGridBackend {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            grid: Grid::new(cols, rows),
            cursor: Position::new(0, 0),
            cursor_visible: true,
            dirty_rows: Vec::new(),
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    /// Resize the underlying grid.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.grid.resize(cols, rows);
        self.cursor = Position::new(
            self.cursor.x.min(cols.saturating_sub(1)),
            self.cursor.y.min(rows.saturating_sub(1)),
        );
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Returns the set of row indices that changed since the last call, and
    /// clears the internal dirty set. An empty return means the grid is
    /// identical to the previous frame — the painter can skip all CPU work and
    /// reuse its cached GPU buffers.
    pub fn take_dirty_rows(&mut self) -> Vec<u16> {
        // Deduplicate: ratatui calls draw() once per changed cell, so a row
        // with many changes appears many times. Sort + dedup is cheap.
        self.dirty_rows.sort_unstable();
        self.dirty_rows.dedup();
        std::mem::take(&mut self.dirty_rows)
    }
}

impl Backend for RatatuiGridBackend {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a RatatuiCell)>,
    {
        for (x, y, rcell) in content {
            self.dirty_rows.push(y);
            self.grid.set(
                x,
                y,
                Cell {
                    symbol: rcell.symbol().to_string(),
                    fg: rcell.fg,
                    bg: rcell.bg,
                    attrs: rcell.modifier,
                },
            );
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = true;
        Ok(())
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(self.cursor)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.cursor = position.into();
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.grid.fill(Cell::default());
        Ok(())
    }

    fn size(&self) -> io::Result<Size> {
        Ok(Size::new(self.grid.cols(), self.grid.rows()))
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: Size::new(self.grid.cols(), self.grid.rows()),
            pixels: Size::new(0, 0),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
