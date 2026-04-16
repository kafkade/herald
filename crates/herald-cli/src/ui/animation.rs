use std::time::{Duration, Instant};

use herald_common::{BOARD_COLS, BOARD_ROWS, BoardState, CellContent, Color};

use super::display_state::{CellDisplayState, DisplayGrid};

/// The ordered character cycle for split-flap boards.
/// Space (blank) at index 0, then A-Z, 0-9, and punctuation.
pub const CHAR_SET: &[char] = &[
    ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '!',
    '@', '#', '$', '%', '&', '(', ')', '-', '+', '=', ';', ':', '\'', '"', ',', '.', '/', '?', '*',
];

/// The ordered color cycle for split-flap color tiles.
pub const COLOR_SET: &[Color] = &[
    Color::Red,
    Color::Orange,
    Color::Yellow,
    Color::Green,
    Color::Blue,
    Color::Violet,
    Color::White,
    Color::Black,
];

/// Find a character's position in `CHAR_SET`.
pub fn char_index(ch: char) -> Option<usize> {
    CHAR_SET.iter().position(|&c| c == ch)
}

/// Compute the intermediate characters when cycling forward through `CHAR_SET`.
///
/// Always cycles forward (real split-flap boards go one direction).
/// Returns the sequence excluding `from` but including `to`.
/// If `from == to`, returns empty.
pub fn cycling_steps(from: char, to: char) -> Vec<char> {
    if from == to {
        return Vec::new();
    }

    let from_idx = match char_index(from) {
        Some(i) => i,
        None => return vec![to], // unknown char → snap
    };
    let to_idx = match char_index(to) {
        Some(i) => i,
        None => return vec![to], // unknown target → snap
    };

    let len = CHAR_SET.len();
    let mut steps = Vec::with_capacity(CHAR_SET.len());
    let mut i = (from_idx + 1) % len;
    loop {
        steps.push(CHAR_SET[i]);
        if i == to_idx {
            break;
        }
        i = (i + 1) % len;
    }
    steps
}

/// Find a color's position in `COLOR_SET`.
pub fn color_index(color: &Color) -> usize {
    COLOR_SET.iter().position(|c| c == color).unwrap_or(0)
}

/// Compute the intermediate colors when cycling forward through `COLOR_SET`.
/// Returns the sequence excluding `from` but including `to`.
pub fn color_cycling_steps(from: &Color, to: &Color) -> Vec<Color> {
    if from == to {
        return Vec::new();
    }
    let from_idx = color_index(from);
    let to_idx = color_index(to);
    let len = COLOR_SET.len();
    let mut steps = Vec::with_capacity(len);
    let mut i = (from_idx + 1) % len;
    loop {
        steps.push(COLOR_SET[i]);
        if i == to_idx {
            break;
        }
        i = (i + 1) % len;
    }
    steps
}

/// Extract the displayable character from a `CellDisplayState`.
/// Color cells are treated as space for animation purposes so they participate in cycling.
fn display_char(state: &CellDisplayState) -> char {
    match state {
        CellDisplayState::Normal(CellContent::Char(c)) => *c,
        CellDisplayState::Normal(CellContent::Blank) => ' ',
        CellDisplayState::Normal(CellContent::Color(_)) => ' ',
        CellDisplayState::Flipping(c) => *c,
        CellDisplayState::FlippingColor(_) => ' ',
    }
}

/// Map a target `CellContent` to the character used in the char set.
/// Color cells are treated as space for animation purposes.
fn target_char(content: &CellContent) -> char {
    match content {
        CellContent::Char(c) => *c,
        CellContent::Blank => ' ',
        CellContent::Color(_) => ' ',
    }
}

/// Check if the display state and target content are actually different,
/// even when their animation characters are the same (e.g., Blank vs Color both map to ' ').
fn from_state_differs_from_target(from: &CellDisplayState, to: &CellContent) -> bool {
    match from {
        CellDisplayState::Normal(content) => content != to,
        CellDisplayState::Flipping(_) | CellDisplayState::FlippingColor(_) => true,
    }
}

/// Extract the color from a display state, if it's a color cell.
fn display_color(state: &CellDisplayState) -> Option<Color> {
    match state {
        CellDisplayState::Normal(CellContent::Color(c)) => Some(*c),
        CellDisplayState::FlippingColor(c) => Some(*c),
        _ => None,
    }
}

/// The kind of cycling animation for a cell.
enum AnimationKind {
    /// Cycling through characters (for Char/Blank cells).
    Char { steps: Vec<char>, from_char: char },
    /// Cycling through colors (for Color cells).
    ColorCycle {
        steps: Vec<Color>,
        from_color: Color,
    },
}

/// Per-cell animation state.
struct CellAnimation {
    kind: AnimationKind,
    /// When this cell's animation was created (used with `delay` to compute start).
    created_at: Instant,
    /// Cascade delay before this cell starts cycling.
    delay: Duration,
    /// The target cell content (for producing the final `CellDisplayState`).
    target: CellContent,
}

impl CellAnimation {
    fn step_count(&self) -> usize {
        match &self.kind {
            AnimationKind::Char { steps, .. } => steps.len(),
            AnimationKind::ColorCycle { steps, .. } => steps.len(),
        }
    }
}

/// Manages the animation of the full board transitioning from one state to another.
pub struct BoardAnimation {
    cells: Vec<Vec<Option<CellAnimation>>>,
    /// When this animation was created — used as the reference point for sampling.
    #[allow(dead_code)] // Accessed in tests for deterministic time-based assertions
    pub(crate) created_at: Instant,
    step_duration: Duration,
    /// The target board state (for cells with no animation).
    target: BoardState,
}

impl BoardAnimation {
    /// Create a new board animation transitioning from the current display to a new target.
    ///
    /// - `from`: the currently rendered `DisplayGrid` (handles mid-animation restarts)
    /// - `to`: the new target `BoardState`
    /// - `step_duration`: time per character step (~50ms)
    /// - `stagger_per_column`: cascade delay between columns (~20ms)
    pub fn new(
        from: &DisplayGrid,
        to: &BoardState,
        step_duration: Duration,
        stagger_per_column: Duration,
    ) -> Self {
        let now = Instant::now();
        let mut cells = Vec::with_capacity(BOARD_ROWS);

        for row in 0..BOARD_ROWS {
            let mut row_cells = Vec::with_capacity(BOARD_COLS);
            for col in 0..BOARD_COLS {
                let from_state = &from.cells[row][col];
                let to_content = &to.grid.0[row][col];

                // Determine if this is a color-to-color transition
                let from_color = display_color(from_state);
                let to_color = match to_content {
                    CellContent::Color(c) => Some(*c),
                    _ => None,
                };

                let anim = match (from_color, to_color) {
                    // Color → Color: cycle through intermediate colors
                    (Some(fc), Some(tc)) if fc != tc => {
                        let steps = color_cycling_steps(&fc, &tc);
                        Some(CellAnimation {
                            kind: AnimationKind::ColorCycle {
                                steps,
                                from_color: fc,
                            },
                            created_at: now,
                            delay: stagger_per_column * col as u32,
                            target: *to_content,
                        })
                    }
                    // Same color → no animation
                    (Some(_), Some(_)) => None,
                    // Non-color → Color: char-cycle to space then snap to color
                    (None, Some(tc)) => {
                        let from_ch = display_char(from_state);
                        if from_ch == ' ' {
                            // Already at space — do a short color ramp from Red to target
                            let steps = color_cycling_steps(&Color::Red, &tc);
                            let mut full = vec![Color::Red];
                            full.extend_from_slice(&steps);
                            Some(CellAnimation {
                                kind: AnimationKind::ColorCycle {
                                    steps: full,
                                    from_color: Color::Red,
                                },
                                created_at: now,
                                delay: stagger_per_column * col as u32,
                                target: *to_content,
                            })
                        } else {
                            // Char → Color: flip through chars to space, then land on color
                            let steps = cycling_steps(from_ch, ' ');
                            Some(CellAnimation {
                                kind: AnimationKind::Char {
                                    steps,
                                    from_char: from_ch,
                                },
                                created_at: now,
                                delay: stagger_per_column * col as u32,
                                target: *to_content,
                            })
                        }
                    }
                    // Color → Non-color: cycle through colors briefly then land on target
                    (Some(fc), None) => {
                        let color_steps: Vec<Color> = COLOR_SET
                            .iter()
                            .cycle()
                            .skip(color_index(&fc) + 1)
                            .take(3)
                            .copied()
                            .collect();
                        Some(CellAnimation {
                            kind: AnimationKind::ColorCycle {
                                steps: color_steps,
                                from_color: fc,
                            },
                            created_at: now,
                            delay: stagger_per_column * col as u32,
                            target: *to_content,
                        })
                    }
                    // Non-color → Non-color: standard char cycling
                    (None, None) => {
                        let from_ch = display_char(from_state);
                        let to_ch = target_char(to_content);
                        if from_ch != to_ch {
                            let steps = cycling_steps(from_ch, to_ch);
                            Some(CellAnimation {
                                kind: AnimationKind::Char {
                                    steps,
                                    from_char: from_ch,
                                },
                                created_at: now,
                                delay: stagger_per_column * col as u32,
                                target: *to_content,
                            })
                        } else if from_state_differs_from_target(from_state, to_content) {
                            Some(CellAnimation {
                                kind: AnimationKind::Char {
                                    steps: vec!['A', to_ch],
                                    from_char: from_ch,
                                },
                                created_at: now,
                                delay: stagger_per_column * col as u32,
                                target: *to_content,
                            })
                        } else {
                            None
                        }
                    }
                };

                row_cells.push(anim);
            }
            cells.push(row_cells);
        }

        Self {
            cells,
            created_at: now,
            step_duration,
            target: to.clone(),
        }
    }

    /// Sample the animation at a given instant, producing the current `DisplayGrid`.
    #[allow(dead_code)] // Kept for backward compatibility; tests use this method
    pub fn sample(&self, now: Instant) -> DisplayGrid {
        let mut grid_cells = Vec::with_capacity(BOARD_ROWS);

        for row in 0..BOARD_ROWS {
            let mut row_cells = Vec::with_capacity(BOARD_COLS);
            for col in 0..BOARD_COLS {
                row_cells.push(self.sample_cell(row, col, now));
            }
            grid_cells.push(row_cells);
        }

        DisplayGrid { cells: grid_cells }
    }

    /// Returns true if there are any animated cells (boards were not identical).
    pub fn has_changes(&self) -> bool {
        self.cells.iter().any(|row| row.iter().any(|c| c.is_some()))
    }

    /// Get the target board state this animation is transitioning to.
    pub fn target(&self) -> &BoardState {
        &self.target
    }

    /// Sample the animation at a given instant, writing into an existing `DisplayGrid`
    /// to avoid per-frame allocation.
    pub fn sample_into(&self, now: Instant, display: &mut DisplayGrid) {
        for row in 0..BOARD_ROWS {
            for col in 0..BOARD_COLS {
                display.set(row, col, self.sample_cell(row, col, now));
            }
        }
    }

    /// Sample a single cell's animation state at a given instant.
    fn sample_cell(&self, row: usize, col: usize, now: Instant) -> CellDisplayState {
        match &self.cells[row][col] {
            None => CellDisplayState::Normal(self.target.grid.0[row][col]),
            Some(anim) => {
                let effective_start = anim.created_at + anim.delay;
                if now < effective_start {
                    // Before cascade delay — show original state
                    match &anim.kind {
                        AnimationKind::Char { from_char, .. } => cell_display_for_char(*from_char),
                        AnimationKind::ColorCycle { from_color, .. } => {
                            CellDisplayState::Normal(CellContent::Color(*from_color))
                        }
                    }
                } else if anim.step_count() == 0 {
                    CellDisplayState::Normal(anim.target)
                } else {
                    let elapsed = now.duration_since(effective_start);
                    let step_idx = (elapsed.as_nanos() / self.step_duration.as_nanos()) as usize;

                    if step_idx >= anim.step_count().saturating_sub(1) {
                        // Animation complete — show final target
                        CellDisplayState::Normal(anim.target)
                    } else {
                        // Mid-cycling
                        match &anim.kind {
                            AnimationKind::Char { steps, .. } => {
                                CellDisplayState::Flipping(steps[step_idx])
                            }
                            AnimationKind::ColorCycle { steps, .. } => {
                                CellDisplayState::FlippingColor(steps[step_idx])
                            }
                        }
                    }
                }
            }
        }
    }

    /// Returns `true` when all cells have finished their animation.
    pub fn is_complete(&self, now: Instant) -> bool {
        for row in &self.cells {
            for anim in row.iter().flatten() {
                let count = anim.step_count();
                if count == 0 {
                    continue;
                }
                let effective_start = anim.created_at + anim.delay;
                if now < effective_start {
                    return false;
                }
                let elapsed = now.duration_since(effective_start);
                let steps_completed = (elapsed.as_nanos() / self.step_duration.as_nanos()) as usize;
                if steps_completed < count.saturating_sub(1) {
                    return false;
                }
            }
        }
        true
    }
}

/// Convert a plain character to a `CellDisplayState::Normal` for display.
fn cell_display_for_char(ch: char) -> CellDisplayState {
    if ch == ' ' {
        CellDisplayState::Normal(CellContent::Blank)
    } else {
        CellDisplayState::Normal(CellContent::Char(ch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use herald_common::{BoardState, Color, Grid};

    #[test]
    fn test_char_index_space() {
        assert_eq!(char_index(' '), Some(0));
    }

    #[test]
    fn test_char_index_letters() {
        assert_eq!(char_index('A'), Some(1));
        assert_eq!(char_index('Z'), Some(26));
    }

    #[test]
    fn test_char_index_digits() {
        assert_eq!(char_index('0'), Some(27));
        assert_eq!(char_index('9'), Some(36));
    }

    #[test]
    fn test_char_index_punctuation() {
        assert_eq!(char_index('!'), Some(37));
        assert_eq!(char_index('*'), Some(CHAR_SET.len() - 1));
    }

    #[test]
    fn test_char_index_unknown() {
        assert_eq!(char_index('€'), None);
    }

    #[test]
    fn test_cycling_steps_adjacent() {
        // A→B returns [B]
        assert_eq!(cycling_steps('A', 'B'), vec!['B']);
    }

    #[test]
    fn test_cycling_steps_wrap() {
        // '*' is last in CHAR_SET, ' ' is first → wraps around
        assert_eq!(cycling_steps('*', ' '), vec![' ']);
    }

    #[test]
    fn test_cycling_steps_same() {
        assert_eq!(cycling_steps('A', 'A'), Vec::<char>::new());
    }

    #[test]
    fn test_cycling_steps_multi() {
        assert_eq!(cycling_steps('A', 'D'), vec!['B', 'C', 'D']);
    }

    #[test]
    fn test_cycling_steps_wrap_multi() {
        // '?' is second-to-last, ' ' is index 0, 'A' is index 1
        let steps = cycling_steps('?', 'A');
        assert_eq!(steps, vec!['*', ' ', 'A']);
    }

    fn make_board_with_char(ch: char) -> BoardState {
        let mut state = BoardState::default();
        state.grid = Grid::blank();
        state.grid.0[0][0] = CellContent::Char(ch);
        state
    }

    fn make_blank_display() -> DisplayGrid {
        DisplayGrid::blank()
    }

    #[test]
    fn test_animation_creates_correctly() {
        let display = make_blank_display();
        let target = make_board_with_char('C');
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Cell (0,0) should have animation: ' ' → 'C' = ['A', 'B', 'C']
        assert!(anim.cells[0][0].is_some());
        let cell_anim = anim.cells[0][0].as_ref().unwrap();
        match &cell_anim.kind {
            AnimationKind::Char { steps, .. } => assert_eq!(*steps, vec!['A', 'B', 'C']),
            _ => panic!("Expected Char animation kind"),
        }
    }

    #[test]
    fn test_animation_unchanged_cells_no_animation() {
        let display = make_blank_display();
        let mut target = BoardState::default();
        target.grid = Grid::blank(); // all blank — same as display
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // All cells should be None (no animation)
        for row in &anim.cells {
            for cell in row {
                assert!(cell.is_none());
            }
        }
    }

    #[test]
    fn test_animation_sample_before_delay() {
        let display = make_blank_display();
        // Put the change at column 5 instead of 0 so there's a cascade delay
        let mut target_col5 = BoardState::default();
        target_col5.grid = Grid::blank();
        target_col5.grid.0[0][5] = CellContent::Char('B');

        let anim = BoardAnimation::new(
            &display,
            &target_col5,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Sample immediately — column 5 delay = 100ms, so it should show original (blank)
        let sampled = anim.sample(anim.created_at + Duration::from_millis(10));
        match &sampled.cells[0][5] {
            CellDisplayState::Normal(CellContent::Blank) => {} // correct
            other => panic!("Expected Normal(Blank) before delay, got {other:?}"),
        }
    }

    #[test]
    fn test_animation_sample_during_cycling() {
        let display = make_blank_display();
        let target = make_board_with_char('D');
        // ' ' → 'D' = ['A', 'B', 'C', 'D'], column 0 has no delay
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // At t=25ms we should be on step 0 ('A'), which is Flipping
        let sampled = anim.sample(anim.created_at + Duration::from_millis(25));
        match &sampled.cells[0][0] {
            CellDisplayState::Flipping('A') => {} // correct
            other => panic!("Expected Flipping('A'), got {other:?}"),
        }

        // At t=75ms we should be on step 1 ('B'), which is Flipping
        let sampled = anim.sample(anim.created_at + Duration::from_millis(75));
        match &sampled.cells[0][0] {
            CellDisplayState::Flipping('B') => {} // correct
            other => panic!("Expected Flipping('B'), got {other:?}"),
        }
    }

    #[test]
    fn test_animation_sample_after_complete() {
        let display = make_blank_display();
        let target = make_board_with_char('B');
        // ' ' → 'B' = ['A', 'B'], column 0 no delay
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // At t=500ms everything should be done
        let sampled = anim.sample(anim.created_at + Duration::from_millis(500));
        match &sampled.cells[0][0] {
            CellDisplayState::Normal(CellContent::Char('B')) => {} // correct
            other => panic!("Expected Normal(Char('B')), got {other:?}"),
        }
    }

    #[test]
    fn test_animation_is_complete() {
        let display = make_blank_display();
        let target = make_board_with_char('B');
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Not complete immediately
        assert!(!anim.is_complete(anim.created_at));

        // Complete after enough time
        assert!(anim.is_complete(anim.created_at + Duration::from_secs(5)));
    }

    #[test]
    fn test_cascade_ordering() {
        let display = make_blank_display();
        let mut target = BoardState::default();
        target.grid = Grid::blank();
        target.grid.0[0][0] = CellContent::Char('A');
        target.grid.0[0][5] = CellContent::Char('A');

        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Column 0 has delay 0ms, column 5 has delay 100ms
        let cell0 = anim.cells[0][0].as_ref().unwrap();
        let cell5 = anim.cells[0][5].as_ref().unwrap();
        assert!(cell0.delay < cell5.delay);
        assert_eq!(cell0.delay, Duration::from_millis(0));
        assert_eq!(cell5.delay, Duration::from_millis(100));
    }

    #[test]
    fn test_color_cells_animate_with_flip() {
        let display = make_blank_display();
        let mut target = BoardState::default();
        target.grid = Grid::blank();
        target.grid.0[0][0] = CellContent::Color(Color::Red);

        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Color cells should now have animation (blank → color both map to ' ',
        // but content differs so a short flip is created)
        assert!(anim.cells[0][0].is_some());

        // After animation completes, should show the color
        let sampled = anim.sample(anim.created_at + Duration::from_secs(5));
        match &sampled.cells[0][0] {
            CellDisplayState::Normal(CellContent::Color(Color::Red)) => {} // correct
            other => panic!("Expected Normal(Color(Red)), got {other:?}"),
        }
    }

    #[test]
    fn test_mid_animation_restart() {
        // Start an animation ' ' → 'D' at column 0
        let display = make_blank_display();
        let target1 = make_board_with_char('D');
        let anim1 = BoardAnimation::new(
            &display,
            &target1,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Sample mid-animation at t=75ms → should be on step 1 ('B')
        let mid_time = anim1.created_at + Duration::from_millis(75);
        let mid_display = anim1.sample(mid_time);

        // Now start a NEW animation from mid-display to target 'A'
        let target2 = make_board_with_char('A');
        let anim2 = BoardAnimation::new(
            &mid_display,
            &target2,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // The animation should start from 'B' (current visible), not from ' '
        let cell_anim = anim2.cells[0][0].as_ref().unwrap();
        match &cell_anim.kind {
            AnimationKind::Char { from_char, steps } => {
                assert_eq!(*from_char, 'B');
                // 'B' → 'A' forward-cycling should go: C, D, E, ..., Z, 0-9, punctuation, ' ', A
                assert_eq!(*steps.last().unwrap(), 'A');
                assert!(steps.len() > 2); // definitely wraps around
            }
            _ => panic!("Expected Char animation kind"),
        }
    }

    #[test]
    fn test_has_changes_when_no_changes() {
        let display = make_blank_display();
        let mut target = BoardState::default();
        target.grid = Grid::blank();
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );
        assert!(!anim.has_changes());
    }

    #[test]
    fn test_has_changes_when_changed() {
        let display = make_blank_display();
        let target = make_board_with_char('A');
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );
        assert!(anim.has_changes());
    }

    #[test]
    fn test_sample_into_matches_sample() {
        let display = make_blank_display();
        let target = make_board_with_char('D');
        let anim = BoardAnimation::new(
            &display,
            &target,
            Duration::from_millis(50),
            Duration::from_millis(20),
        );

        // Sample at several time points and verify sample_into gives the same result
        for offset_ms in [0, 25, 75, 150, 500] {
            let t = anim.created_at + Duration::from_millis(offset_ms);
            let from_sample = anim.sample(t);
            let mut into_buf = make_blank_display();
            anim.sample_into(t, &mut into_buf);

            for r in 0..BOARD_ROWS {
                for c in 0..BOARD_COLS {
                    assert_eq!(
                        format!("{:?}", from_sample.cells[r][c]),
                        format!("{:?}", into_buf.cells[r][c]),
                        "mismatch at ({r},{c}) t={offset_ms}ms"
                    );
                }
            }
        }
    }

    #[test]
    fn test_fill_from_board_state_reuses_allocation() {
        let mut display = make_blank_display();
        let ptr_before = display.cells.as_ptr();

        let target = make_board_with_char('Z');
        display.fill_from_board_state(&target);

        // Vec should not have reallocated
        assert_eq!(display.cells.as_ptr(), ptr_before);
        match &display.cells[0][0] {
            CellDisplayState::Normal(CellContent::Char('Z')) => {}
            other => panic!("Expected Normal(Char('Z')), got {other:?}"),
        }
    }
}
