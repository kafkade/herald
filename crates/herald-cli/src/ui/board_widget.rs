use herald_common::{BOARD_COLS, BOARD_ROWS, CellContent, Color};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Position, Rect},
    style::{Color as RatatuiColor, Modifier, Style},
    text::Text,
    widgets::{Paragraph, Widget},
};

use super::display_state::{CellDisplayState, DisplayGrid};

const CELL_WIDTH: u16 = 3;
const GRID_WIDTH: u16 = BOARD_COLS as u16 * CELL_WIDTH + BOARD_COLS as u16 + 1; // 89
const GRID_HEIGHT: u16 = BOARD_ROWS as u16 + BOARD_ROWS as u16 + 1; // 13

/// Maps a Herald color to an ANSI 256-color indexed value.
fn map_color_256(color: &Color) -> RatatuiColor {
    match color {
        Color::Red => RatatuiColor::Indexed(196),
        Color::Orange => RatatuiColor::Indexed(208),
        Color::Yellow => RatatuiColor::Indexed(226),
        Color::Green => RatatuiColor::Indexed(46),
        Color::Blue => RatatuiColor::Indexed(21),
        Color::Violet => RatatuiColor::Indexed(93),
        Color::White => RatatuiColor::Indexed(231),
        Color::Black => RatatuiColor::Indexed(232),
    }
}

/// Basic 8-color fallback for terminals without 256-color support.
fn map_color_basic(color: &Color) -> RatatuiColor {
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

/// Maps a Herald color to a terminal color, with fallback for basic terminals.
fn map_color_with_fallback(color: &Color, use_256: bool) -> RatatuiColor {
    if use_256 {
        map_color_256(color)
    } else {
        map_color_basic(color)
    }
}

/// Detects whether the terminal supports 256 colors via environment variables.
fn supports_256_colors() -> bool {
    if let Ok(colorterm) = std::env::var("COLORTERM")
        && (colorterm == "truecolor" || colorterm == "24bit")
    {
        return true;
    }
    if let Ok(term) = std::env::var("TERM") {
        return term.contains("256color") || term.contains("xterm") || term.contains("screen");
    }
    // Default to 256 on modern systems
    true
}

/// A ratatui widget that renders the Herald 6×22 board grid with box-drawing borders.
pub struct BoardWidget<'a> {
    display: &'a DisplayGrid,
}

impl<'a> BoardWidget<'a> {
    pub fn new(display: &'a DisplayGrid) -> Self {
        Self { display }
    }
}

impl<'a> Widget for BoardWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Show a warning if the terminal is too small.
        if area.width < GRID_WIDTH || area.height < GRID_HEIGHT {
            let msg = format!(
                "Terminal too small. Minimum: {}×{} (have {}×{})",
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
                let display_cell = &self.display.cells[row as usize][col as usize];

                match display_cell {
                    CellDisplayState::Normal(content) => match content {
                        CellContent::Char(c) => {
                            // " c " — character centered in 3 cells
                            let chars = [' ', *c, ' '];
                            for (dx, ch) in chars.iter().enumerate() {
                                if let Some(cell) =
                                    buf.cell_mut(Position::new(x_start + dx as u16, y))
                                {
                                    cell.set_char(*ch);
                                }
                            }
                        }
                        CellContent::Color(color) => {
                            let use_256 = supports_256_colors();
                            let bg = map_color_with_fallback(color, use_256);
                            let style = match color {
                                Color::White => {
                                    Style::default().bg(bg).fg(RatatuiColor::Indexed(232))
                                }
                                Color::Black => {
                                    Style::default().bg(bg).fg(RatatuiColor::Indexed(231))
                                }
                                _ => Style::default().bg(bg),
                            };
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
                    },
                    CellDisplayState::Flipping(ch) => {
                        // "─X─" with dim style on the flap dashes
                        let dim = Style::default().add_modifier(Modifier::DIM);
                        if let Some(cell) = buf.cell_mut(Position::new(x_start, y)) {
                            cell.set_char('─').set_style(dim);
                        }
                        if let Some(cell) = buf.cell_mut(Position::new(x_start + 1, y)) {
                            cell.set_char(*ch);
                        }
                        if let Some(cell) = buf.cell_mut(Position::new(x_start + 2, y)) {
                            cell.set_char('─').set_style(dim);
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
    use herald_common::BoardState;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn blank_display() -> DisplayGrid {
        DisplayGrid::from_board_state(&BoardState::default())
    }

    #[test]
    fn too_small_shows_warning() {
        let dg = blank_display();
        let widget = BoardWidget::new(&dg);
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
        let dg = blank_display();
        let widget = BoardWidget::new(&dg);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        assert_eq!(buf.cell(Position::new(0, 0)).unwrap().symbol(), "┌");
        assert_eq!(
            buf.cell(Position::new(GRID_WIDTH - 1, 0)).unwrap().symbol(),
            "┐"
        );
        assert_eq!(
            buf.cell(Position::new(0, GRID_HEIGHT - 1))
                .unwrap()
                .symbol(),
            "└"
        );
        assert_eq!(
            buf.cell(Position::new(GRID_WIDTH - 1, GRID_HEIGHT - 1))
                .unwrap()
                .symbol(),
            "┘"
        );
    }

    #[test]
    fn renders_char_cell() {
        let mut dg = blank_display();
        dg.cells[0][0] = CellDisplayState::Normal(CellContent::Char('H'));
        let widget = BoardWidget::new(&dg);
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
        let mut dg = blank_display();
        dg.cells[0][0] = CellDisplayState::Normal(CellContent::Color(Color::Red));
        let widget = BoardWidget::new(&dg);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        let cell = buf.cell(Position::new(1, 1)).unwrap();
        // With 256-color support (default), Red maps to Indexed(196)
        assert_eq!(cell.bg, RatatuiColor::Indexed(196));
    }

    #[test]
    fn renders_white_color_cell_with_dark_foreground() {
        let mut dg = blank_display();
        dg.cells[0][0] = CellDisplayState::Normal(CellContent::Color(Color::White));
        let widget = BoardWidget::new(&dg);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        let cell = buf.cell(Position::new(1, 1)).unwrap();
        assert_eq!(cell.bg, RatatuiColor::Indexed(231));
        assert_eq!(cell.fg, RatatuiColor::Indexed(232));
    }

    #[test]
    fn renders_black_color_cell_with_white_foreground() {
        let mut dg = blank_display();
        dg.cells[0][0] = CellDisplayState::Normal(CellContent::Color(Color::Black));
        let widget = BoardWidget::new(&dg);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        let cell = buf.cell(Position::new(1, 1)).unwrap();
        assert_eq!(cell.bg, RatatuiColor::Indexed(232));
        assert_eq!(cell.fg, RatatuiColor::Indexed(231));
    }

    #[test]
    fn renders_flipping_cell() {
        let mut dg = blank_display();
        dg.cells[0][0] = CellDisplayState::Flipping('A');
        let widget = BoardWidget::new(&dg);
        let area = Rect::new(0, 0, GRID_WIDTH, GRID_HEIGHT);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        // Cell (0,0) content starts at x=1, y=1 → should be "─A─"
        assert_eq!(buf.cell(Position::new(1, 1)).unwrap().symbol(), "─");
        assert_eq!(buf.cell(Position::new(2, 1)).unwrap().symbol(), "A");
        assert_eq!(buf.cell(Position::new(3, 1)).unwrap().symbol(), "─");

        // The ─ characters should have DIM modifier
        let left = buf.cell(Position::new(1, 1)).unwrap();
        assert!(
            left.modifier.contains(Modifier::DIM),
            "left dash should be DIM"
        );
        let right = buf.cell(Position::new(3, 1)).unwrap();
        assert!(
            right.modifier.contains(Modifier::DIM),
            "right dash should be DIM"
        );
    }
}
