use chrono::{DateTime, Utc};

use crate::{CellContent, Countdown, Grid, BOARD_COLS, BOARD_ROWS};

/// Render a countdown onto the 6×22 board grid.
///
/// Layout:
/// - Rows 0-1: countdown label (centered, split at 22 chars)
/// - Row 2: blank
/// - Rows 3-4: formatted time remaining (centered, split on "/")
/// - Row 5: blank
pub fn render_countdown_grid(countdown: &Countdown, now: DateTime<Utc>) -> Grid {
    let mut grid = Grid::blank();

    // ── Label on rows 0–1 ────────────────────────────────
    let label_chars: Vec<char> = countdown.label.chars().collect();
    let row0: Vec<char> = label_chars.iter().take(BOARD_COLS).copied().collect();
    place_text_centered(&mut grid, 0, &row0);

    if label_chars.len() > BOARD_COLS {
        let row1: Vec<char> = label_chars[BOARD_COLS..]
            .iter()
            .take(BOARD_COLS)
            .copied()
            .collect();
        place_text_centered(&mut grid, 1, &row1);
    }

    // ── Time remaining on rows 3–4 ──────────────────────
    let remaining = if countdown.target > now {
        countdown.target - now
    } else {
        chrono::Duration::zero()
    };

    let formatted = format_template(&countdown.format_template, remaining);

    // Split on "/" for multi-row time display
    let parts: Vec<&str> = formatted.splitn(2, '/').collect();
    let time_row0: Vec<char> = parts[0].trim().chars().take(BOARD_COLS).collect();
    place_text_centered(&mut grid, 3, &time_row0);

    if parts.len() > 1 {
        let time_row1: Vec<char> = parts[1].trim().chars().take(BOARD_COLS).collect();
        place_text_centered(&mut grid, 4, &time_row1);
    }

    grid
}

/// Replace format template placeholders with computed time values.
fn format_template(template: &str, duration: chrono::Duration) -> String {
    let total_secs = duration.num_seconds().max(0);
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    // Replace longest tokens first to avoid partial matches
    template
        .replace("{DDD}", &format!("{days:03}"))
        .replace("{DD}", &format!("{days:02}"))
        .replace("{D}", &days.to_string())
        .replace("{HH}", &format!("{hours:02}"))
        .replace("{H}", &hours.to_string())
        .replace("{MM}", &format!("{mins:02}"))
        .replace("{M}", &mins.to_string())
        .replace("{SS}", &format!("{secs:02}"))
        .replace("{S}", &secs.to_string())
}

/// Place characters centered on a grid row.
fn place_text_centered(grid: &mut Grid, row: usize, chars: &[char]) {
    if row >= BOARD_ROWS || chars.is_empty() {
        return;
    }
    let len = chars.len().min(BOARD_COLS);
    let start = (BOARD_COLS - len) / 2;
    for (i, &ch) in chars[..len].iter().enumerate() {
        grid.0[row][start + i] = CellContent::Char(ch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ZeroBehavior;
    use uuid::Uuid;

    fn make_countdown(label: &str, target: DateTime<Utc>, template: &str) -> Countdown {
        Countdown {
            id: Uuid::new_v4(),
            label: label.to_string(),
            target,
            format_template: template.to_string(),
            zero_behavior: ZeroBehavior::ShowZero,
            queue_position: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_format_template_basic() {
        let dur = chrono::Duration::seconds(42 * 86400 + 7 * 3600 + 31 * 60 + 15);
        let result = format_template("{DDD} DAYS  {HH}:{MM}:{SS}", dur);
        assert_eq!(result, "042 DAYS  07:31:15");
    }

    #[test]
    fn test_format_template_no_leading_zeros() {
        let dur = chrono::Duration::seconds(5 * 86400 + 3 * 3600 + 9 * 60 + 2);
        let result = format_template("{D} DAYS  {H}:{M}:{S}", dur);
        assert_eq!(result, "5 DAYS  3:9:2");
    }

    #[test]
    fn test_format_template_zero_duration() {
        let dur = chrono::Duration::zero();
        let result = format_template("{DDD} DAYS  {HH}:{MM}:{SS}", dur);
        assert_eq!(result, "000 DAYS  00:00:00");
    }

    #[test]
    fn test_format_template_over_99_days() {
        let dur = chrono::Duration::seconds(365 * 86400 + 12 * 3600);
        let result = format_template("{D} DAYS  {HH}:{MM}:{SS}", dur);
        assert_eq!(result, "365 DAYS  12:00:00");
    }

    #[test]
    fn test_format_template_ddd_over_999() {
        let dur = chrono::Duration::seconds(1234 * 86400);
        let result = format_template("{DDD} DAYS", dur);
        assert_eq!(result, "1234 DAYS");
    }

    #[test]
    fn test_render_countdown_label_placed_on_row_0() {
        let now = Utc::now();
        let target = now + chrono::Duration::hours(1);
        let cd = make_countdown("LAUNCH", target, "{HH}:{MM}:{SS}");
        let grid = render_countdown_grid(&cd, now);

        let start = (BOARD_COLS - 6) / 2; // 8
        assert_eq!(grid.0[0][start], CellContent::Char('L'));
        assert_eq!(grid.0[0][start + 1], CellContent::Char('A'));
        assert_eq!(grid.0[0][start + 5], CellContent::Char('H'));
    }

    #[test]
    fn test_render_countdown_time_on_row_3() {
        let now = Utc::now();
        let target = now + chrono::Duration::seconds(3661); // 1h 1m 1s
        let cd = make_countdown("TEST", target, "{HH}:{MM}:{SS}");
        let grid = render_countdown_grid(&cd, now);

        let time_str = "01:01:01";
        let start = (BOARD_COLS - time_str.len()) / 2; // 7
        for (i, ch) in time_str.chars().enumerate() {
            assert_eq!(
                grid.0[3][start + i],
                CellContent::Char(ch),
                "mismatch at row 3, col {}",
                start + i
            );
        }
    }

    #[test]
    fn test_render_countdown_past_target_shows_zero() {
        let now = Utc::now();
        let target = now - chrono::Duration::hours(1);
        let cd = make_countdown("DONE", target, "{HH}:{MM}:{SS}");
        let grid = render_countdown_grid(&cd, now);

        let time_str = "00:00:00";
        let start = (BOARD_COLS - time_str.len()) / 2;
        for (i, ch) in time_str.chars().enumerate() {
            assert_eq!(grid.0[3][start + i], CellContent::Char(ch));
        }
    }

    #[test]
    fn test_render_countdown_row_2_and_5_blank() {
        let now = Utc::now();
        let target = now + chrono::Duration::hours(1);
        let cd = make_countdown("TEST", target, "{HH}:{MM}:{SS}");
        let grid = render_countdown_grid(&cd, now);

        for col in 0..BOARD_COLS {
            assert_eq!(grid.0[2][col], CellContent::Blank, "row 2, col {col} not blank");
            assert_eq!(grid.0[5][col], CellContent::Blank, "row 5, col {col} not blank");
        }
    }

    #[test]
    fn test_render_countdown_multirow_time() {
        let now = Utc::now();
        let target = now + chrono::Duration::seconds(42 * 86400 + 7 * 3600 + 31 * 60 + 15);
        let cd = make_countdown("EVENT", target, "{D} DAYS  {H} HRS/{M} MIN   {S} SEC");
        let grid = render_countdown_grid(&cd, now);

        let row3_has_content = grid.0[3].iter().any(|c| *c != CellContent::Blank);
        let row4_has_content = grid.0[4].iter().any(|c| *c != CellContent::Blank);
        assert!(row3_has_content, "row 3 should have time content");
        assert!(row4_has_content, "row 4 should have time content");
    }

    #[test]
    fn test_render_countdown_long_label_wraps_to_row_1() {
        let now = Utc::now();
        let target = now + chrono::Duration::hours(1);
        let cd = make_countdown("ABCDEFGHIJKLMNOPQRSTUV12345678", target, "{HH}:{MM}:{SS}");
        let grid = render_countdown_grid(&cd, now);

        let row0_has_content = grid.0[0].iter().any(|c| *c != CellContent::Blank);
        assert!(row0_has_content, "row 0 should have label content");

        let row1_has_content = grid.0[1].iter().any(|c| *c != CellContent::Blank);
        assert!(row1_has_content, "row 1 should have label overflow");
    }
}
