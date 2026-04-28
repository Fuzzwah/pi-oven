//! pi-oven renderer: cell grid, ratatui backend, wgpu + glyphon paint pipeline.
//!
//! Sections 12 and 13 of the `scaffold-runtime` change land here. The grid and
//! backend are reusable on their own (see the `dev-crossterm` feature in the
//! binary, which intentionally bypasses everything in this crate).

pub mod backend;
pub mod grid;
pub mod paint;

pub use backend::RatatuiGridBackend;
pub use grid::{Attrs, Cell, Grid};
pub use paint::{CellMetrics, ClearColor, Painter};
