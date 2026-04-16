use herald_common::{BOARD_COLS, BOARD_ROWS, BoardState, CellContent, Color};

/// What to display in a single cell during rendering.
#[derive(Clone, Debug)]
pub enum CellDisplayState {
    /// Show normal content from the grid.
    Normal(CellContent),
    /// Show a character mid-flip (the intermediate cycling character, with flap visual).
    Flipping(char),
    /// Show a color tile mid-flip (cycling through intermediate colors).
    FlippingColor(Color),
}

/// The display state for the full board — drives what BoardWidget actually renders.
#[derive(Clone, Debug)]
pub struct DisplayGrid {
    pub cells: Vec<Vec<CellDisplayState>>,
}

impl DisplayGrid {
    /// Convert a `BoardState` into a `DisplayGrid` with all cells set to `Normal`.
    pub fn from_board_state(state: &BoardState) -> Self {
        let cells = state
            .grid
            .0
            .iter()
            .map(|row| row.iter().map(|c| CellDisplayState::Normal(*c)).collect())
            .collect();
        Self { cells }
    }

    /// Create a blank display grid (all `Normal(Blank)` cells).
    #[allow(dead_code)] // Public API used by tests across modules
    pub fn blank() -> Self {
        Self {
            cells: vec![vec![CellDisplayState::Normal(CellContent::Blank); BOARD_COLS]; BOARD_ROWS],
        }
    }

    /// Set a cell in the display grid.
    pub fn set(&mut self, row: usize, col: usize, state: CellDisplayState) {
        self.cells[row][col] = state;
    }

    /// Overwrite every cell from a `BoardState`, reusing the existing allocation.
    pub fn fill_from_board_state(&mut self, state: &BoardState) {
        for (r, row) in state.grid.0.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                self.cells[r][c] = CellDisplayState::Normal(*cell);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_board_state_produces_all_normal() {
        let mut state = BoardState::default();
        state.grid.0[0][0] = CellContent::Char('Z');
        state.grid.0[1][2] = CellContent::Color(herald_common::Color::Red);

        let dg = DisplayGrid::from_board_state(&state);
        assert_eq!(dg.cells.len(), BOARD_ROWS);
        for row in &dg.cells {
            assert_eq!(row.len(), BOARD_COLS);
        }

        // Every cell should be Normal
        for (r, row) in dg.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                match cell {
                    CellDisplayState::Normal(_) => {} // ok
                    other => {
                        panic!("Expected Normal at ({r},{c}), got {other:?}");
                    }
                }
            }
        }

        // Spot-check preserved content
        match &dg.cells[0][0] {
            CellDisplayState::Normal(CellContent::Char('Z')) => {}
            other => panic!("Expected Normal(Char('Z')), got {other:?}"),
        }
        match &dg.cells[1][2] {
            CellDisplayState::Normal(CellContent::Color(herald_common::Color::Red)) => {}
            other => panic!("Expected Normal(Color(Red)), got {other:?}"),
        }
    }
}
