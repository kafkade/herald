pub mod countdown;
pub mod types;

pub use countdown::render_countdown_grid;
pub use types::*;

/// Board dimensions: 6 rows × 22 columns.
pub const BOARD_ROWS: usize = 6;
pub const BOARD_COLS: usize = 22;

/// Generate the default splash screen grid with "HERALD" centered.
/// Displayed when the queue is empty (spec §5.3).
pub fn splash_grid() -> Grid {
    let mut grid = Grid::blank();
    let letters = ['H', 'E', 'R', 'A', 'L', 'D'];
    let text_with_spaces: Vec<char> = letters
        .iter()
        .enumerate()
        .flat_map(|(i, &c)| if i > 0 { vec![' ', c] } else { vec![c] })
        .collect();
    let start = (BOARD_COLS - text_with_spaces.len()) / 2;
    let row = 2;

    for (i, &ch) in text_with_spaces.iter().enumerate() {
        if ch != ' ' {
            grid.0[row][start + i] = CellContent::Char(ch);
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_grid_contains_herald() {
        let grid = splash_grid();
        let row2_chars: String = grid.0[2]
            .iter()
            .filter_map(|c| match c {
                CellContent::Char(ch) => Some(*ch),
                _ => None,
            })
            .collect();
        assert_eq!(row2_chars, "HERALD");
    }

    #[test]
    fn splash_grid_other_rows_blank() {
        let grid = splash_grid();
        for row_idx in [0, 1, 3, 4, 5] {
            for cell in &grid.0[row_idx] {
                assert_eq!(*cell, CellContent::Blank, "row {row_idx} should be blank");
            }
        }
    }
}
