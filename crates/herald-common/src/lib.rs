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

    /// Extract all chars from a grid row.
    fn row_text(grid: &Grid, row: usize) -> String {
        grid.0[row]
            .iter()
            .filter_map(|c| match c {
                CellContent::Char(ch) => Some(*ch),
                _ => None,
            })
            .collect()
    }

    /// Extract chars from a grid row preserving blanks as spaces (trimmed at end).
    fn row_text_with_blanks(grid: &Grid, row: usize) -> String {
        let s: String = grid.0[row]
            .iter()
            .map(|c| match c {
                CellContent::Char(ch) => *ch,
                _ => ' ',
            })
            .collect();
        s.trim_end().to_string()
    }

    #[test]
    fn from_text_simple_centered() {
        let grid = Grid::from_text("HELLO", HAlign::Center, VAlign::Middle).unwrap();
        // "HELLO" is 5 chars, centered in 22 cols → starts at col 8
        // Vertically middle with 1 line in 6 rows → row 2
        assert_eq!(row_text(&grid, 2), "HELLO");
        // Other rows should be blank
        for r in [0, 1, 3, 4, 5] {
            assert_eq!(row_text(&grid, r), "");
        }
    }

    #[test]
    fn from_text_left_aligned() {
        let grid = Grid::from_text("HI", HAlign::Left, VAlign::Top).unwrap();
        let row = row_text_with_blanks(&grid, 0);
        assert!(row.starts_with("HI"));
    }

    #[test]
    fn from_text_right_aligned() {
        let grid = Grid::from_text("HI", HAlign::Right, VAlign::Top).unwrap();
        let row = row_text_with_blanks(&grid, 0);
        assert!(row.ends_with("HI"));
    }

    #[test]
    fn from_text_uppercases() {
        let grid = Grid::from_text("hello", HAlign::Center, VAlign::Middle).unwrap();
        assert_eq!(row_text(&grid, 2), "HELLO");
    }

    #[test]
    fn from_text_word_wraps() {
        // "AAAA BBBB CCCC DDDD EEEE FFFF" → wraps to multiple lines
        let grid =
            Grid::from_text("AAAA BBBB CCCC DDDD EEEE FFFF", HAlign::Center, VAlign::Top).unwrap();
        // First line should have some words, second line the rest
        let line0 = row_text(&grid, 0);
        let line1 = row_text(&grid, 1);
        assert!(!line0.is_empty());
        assert!(!line1.is_empty());
    }

    #[test]
    fn from_text_explicit_newlines() {
        let grid = Grid::from_text("LINE ONE\nLINE TWO", HAlign::Center, VAlign::Top).unwrap();
        // Spaces are valid chars on the board, so row_text includes them
        assert_eq!(row_text(&grid, 0), "LINE ONE");
        assert_eq!(row_text(&grid, 1), "LINE TWO");
    }

    #[test]
    fn from_text_overflow_returns_error() {
        // 7 lines should fail (board has 6 rows)
        let text = "A\nB\nC\nD\nE\nF\nG";
        let result = Grid::from_text(text, HAlign::Center, VAlign::Top);
        assert!(result.is_err());
    }

    #[test]
    fn from_text_empty_returns_blank_grid() {
        let grid = Grid::from_text("", HAlign::Center, VAlign::Middle).unwrap();
        assert_eq!(grid, Grid::blank());
    }

    #[test]
    fn from_text_normalizes_unsupported_chars() {
        // Unsupported chars become blanks (spaces)
        let grid = Grid::from_text("HI™THERE", HAlign::Left, VAlign::Top).unwrap();
        let row = row_text(&grid, 0);
        // ™ is unsupported → space → words become "HI THERE"
        assert!(row.contains("HI"));
        assert!(row.contains("THERE"));
    }

    #[test]
    fn from_text_long_word_hard_splits() {
        let long = "A".repeat(30); // longer than 22 cols
        let grid = Grid::from_text(&long, HAlign::Left, VAlign::Top).unwrap();
        // Should be split into 22 + 8 chars across 2 rows
        assert_eq!(row_text(&grid, 0).len(), 22);
        assert_eq!(row_text(&grid, 1).len(), 8);
    }

    #[test]
    fn from_text_max_rows_ok() {
        // Exactly 6 lines should succeed
        let text = "A\nB\nC\nD\nE\nF";
        let grid = Grid::from_text(text, HAlign::Center, VAlign::Top).unwrap();
        for r in 0..6 {
            assert!(!row_text(&grid, r).is_empty());
        }
    }
}
