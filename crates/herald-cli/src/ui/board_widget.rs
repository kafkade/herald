use herald_common::{BOARD_COLS, BOARD_ROWS, BoardState, CellContent, Color};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Position, Rect},
    style::{Color as RatatuiColor, Style},
    text::Text,
    widgets::{Paragraph, Widget},
};

const CELL_WIDTH: u16 = 3;
const GRID_WIDTH: u16 = BOARD_COLS as u16 * CELL_WIDTH + BOARD_COLS as u16 + 1; // 89
const GRID_HEIGHT: u16 = BOARD_ROWS as u16 + BOARD_ROWS as u16 + 1; // 13

/// Maps a Herald color to a ratatui terminal color.
fn map_color(color: &Color) -> RatatuiColor {
    match color {
        Color::Red => RatatuiColor::Red,
        Color::Orange => RatatuiColor::Rgb(255, 165, 0),
        Color::Yellow => RatatuiColor::Yellow,
        Color::Green => RatatuiColor::Green,
        Color::Blue => RatatuiColor::Blue,
        Color::Violet => RatatuiColor::Magenta,
        Color::White => RatatuiColor::White,
        Color::Black => RatatuiColor::Black,
    }
}

/// A ratatui widget that renders the Herald 6×22 board grid with box-drawing borders.
pub struct BoardWidget<'a> {
    board_state: &'a BoardState,
}

impl<'a> BoardWidget<'a> {
    pub fn new(board_state: &'a BoardState) -> Self {
        Self { board_state }
    }
}

impl<'a> Widget for BoardWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Show a warning if the terminal is too small.
        if area.width < GRID_WIDTH || area.height < GRID_HEIGHT {
            let msg = format!(
                "Terminal too small (need {}×{}, have {}×{})",
                GRID_WIDTH, GRID_HEIGHT, area.width, area.height
            );
            let paragraph = Paragraph::new(Text::raw(msg)).alignment(Alignment::Center);
            // Vertically center the single-line warning.
            let y_off = area.height.saturating_sub(1) / 2;
            let warning_area = Rect::new(area.x, area.y + y_off, area.width, 1);
            paragraph.render(warning_area, buf);
            return;
        }

        let x_offset = area.x + (area.width.saturating_sub(GRID_WIDTH)) / 2;
        let y_offset = area.y + (area.height.saturating_sub(GRID_HEIGHT)) / 2;

        // ── Draw horizontal border rows ──────────────────────────────
        for row in 0..=BOARD_ROWS as u16 {
            let y = y_offset + row * 2;
            for col in 0..=BOARD_COLS as u16 {
                let x = x_offset + col * (CELL_WIDTH + 1);

                // Junction character
                let junction = match (row, col) {
                    (0, 0) => '┌',
                    (0, c) if c == BOARD_COLS as u16 => '┐',
                    (r, 0) if r == BOARD_ROWS as u16 => '└',
                    (r, c) if r == BOARD_ROWS as u16 && c == BOARD_COLS as u16 => '┘',
                    (0, _) => '┬',
                    (r, _) if r == BOARD_ROWS as u16 => '┴',
                    (_, 0) => '├',
                    (_, c) if c == BOARD_COLS as u16 => '┤',
                    _ => '┼',
                };

                if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                    cell.set_char(junction);
                }

                // Horizontal dashes after the junction (except at last column)
                if col < BOARD_COLS as u16 {
                    for dx in 1..=CELL_WIDTH {
                        if let Some(cell) = buf.cell_mut(Position::new(x + dx, y)) {
                            cell.set_char('─');
                        }
                    }
                }
            }
        }

        // ── Draw content rows ────────────────────────────────────────
        for row in 0..BOARD_ROWS as u16 {
            let y = y_offset + row * 2 + 1;

            // Vertical borders (including right-most)
            for col in 0..=BOARD_COLS as u16 {
                let x = x_offset + col * (CELL_WIDTH + 1);
                if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                    cell.set_char('│');
                }
            }

            // Cell content
            for col in 0..BOARD_COLS as u16 {
                let x_start = x_offset + col * (CELL_WIDTH + 1) + 1;
                let content = &self.board_state.grid.0[row as usize][col as usize];

                match content {
                    CellContent::Char(c) => {
                        // " c " — character centered in 3 cells
                        let chars = [' ', *c, ' '];
                        for (dx, ch) in chars.iter().enumerate() {
                            if let Some(cell) = buf.cell_mut(Position::new(x_start + dx as u16, y))
                            {
                                cell.set_char(*ch);
                            }
                        }
                    }
                    CellContent::Color(color) => {
                        let style = Style::default().bg(map_color(color));
                        for dx in 0..CELL_WIDTH {
                            if let Some(cell) = buf.cell_mut(Position::new(x_start + dx, y)) {
                                cell.set_char(' ').set_style(style);
                            }
                        }
                    }
                    CellContent::Blank => {
                        for dx in 0..CELL_WIDTH {
                            if let Some(cell) = buf.cell_mut(Position::new(x_start + dx, y)) {
                                cell.set_char(' ');
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn blank_board() -> BoardState {
        BoardState::default()
    }

    #[test]
    fn too_small_shows_warning() {
        let state = blank_board();
        let widget = BoardWidget::new(&state);
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        let text: String = (0..area.width)
            .map(|x| {
                buf.cell(Position::new(x, 2))
                    .unwrap()
                    .symbol()
                    .chars()
                    .next()
                    .unwrap_or(' ')
            })
            .collect();
        assert!(text.contains("Terminal too small"));
    }

    #[test]
    fn renders_corners() {
        let state = blank_board();
        let widget = BoardWidget::new(&state);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        // Top-left corner
        assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "┌");
        // Top-right corner
        assert_eq!(
            buf.cell(Position::new(GRID_WIDTH - 1, 0)).unwrap().symbol(),
            "┐"
        );
        // Bottom-left corner
        assert_eq!(
            buf.cell(Position::new(0, GRID_HEIGHT - 1))
                .unwrap()
                .symbol(),
            "└"
        );
        // Bottom-right corner
        assert_eq!(
            buf.cell(Position::new(GRID_WIDTH - 1, GRID_HEIGHT - 1))
                .unwrap()
                .symbol(),
            "┘"
        );
    }

    #[test]
    fn renders_char_cell() {
        let mut state = blank_board();
        state.grid.0[0][0] = CellContent::Char('H');
        let widget = BoardWidget::new(&state);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        // Cell (0,0) content starts at x=1, y=1
        assert_eq!(buf.cell(Position::new(1, 1)).unwrap().symbol(), " ");
        assert_eq!(buf.cell(Position::new(2, 1)).unwrap().symbol(), "H");
        assert_eq!(buf.cell(Position::new(3, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn renders_color_cell_background() {
        let mut state = blank_board();
        state.grid.0[0][0] = CellContent::Color(Color::Red);
        let widget = BoardWidget::new(&state);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        let cell = buf.cell(Position::new(1, 1)).unwrap();
        assert_eq!(cell.bg, RatatuiColor::Red);
    }
}
