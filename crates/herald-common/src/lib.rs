pub mod countdown;
pub mod types;

pub use countdown::render_countdown_grid;
pub use types::*;

/// Board dimensions: 6 rows × 22 columns.
pub const BOARD_ROWS: usize = 6;
pub const BOARD_COLS: usize = 22;
