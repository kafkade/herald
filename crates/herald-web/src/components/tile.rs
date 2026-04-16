use herald_common::{CellContent, Color};
use leptos::prelude::*;

/// Convert a CellContent to its display character.
fn cell_to_char(cell: &CellContent) -> String {
    match cell {
        CellContent::Char(c) => c.to_string(),
        CellContent::Blank => " ".to_string(),
        CellContent::Color(_) => String::new(),
    }
}

/// Get CSS background color for a Color tile.
fn color_to_css(color: &Color) -> &'static str {
    match color {
        Color::Red => "var(--color-red)",
        Color::Orange => "var(--color-orange)",
        Color::Yellow => "var(--color-yellow)",
        Color::Green => "var(--color-green)",
        Color::Blue => "var(--color-blue)",
        Color::Violet => "var(--color-violet)",
        Color::White => "var(--color-white)",
        Color::Black => "var(--color-black)",
    }
}

/// Whether a color tile needs dark text (for light backgrounds).
fn is_light_color(color: &Color) -> bool {
    matches!(color, Color::White | Color::Yellow)
}

/// A single split-flap tile component.
///
/// Renders the top/bottom halves of a mechanical flap tile with 3D flip animation.
/// Uses `--col-index` CSS variable for cascade stagger delay.
#[component]
pub fn FlapTile(
    cell: RwSignal<CellContent>,
    prev_cell: RwSignal<CellContent>,
    col_index: usize,
    update_counter: RwSignal<u64>,
    has_received_update: RwSignal<bool>,
) -> impl IntoView {
    let is_animating = RwSignal::new(false);

    // When update_counter changes, check if this cell actually changed and trigger animation
    Effect::new(move |prev_count: Option<u64>| {
        let count = update_counter.get();
        if prev_count.is_some() {
            // Subsequent updates: only animate changed cells
            let current = cell.get();
            let previous = prev_cell.get();
            if current != previous {
                is_animating.set(true);
                let delay_ms = (col_index as u32) * 20 + 350;
                set_timeout(
                    move || is_animating.set(false),
                    std::time::Duration::from_millis(delay_ms as u64),
                );
            }
        } else if has_received_update.get() {
            // First update: animate all tiles from loading state
            is_animating.set(true);
            let delay_ms = (col_index as u32) * 20 + 350;
            set_timeout(
                move || is_animating.set(false),
                std::time::Duration::from_millis(delay_ms as u64),
            );
        }
        count
    });

    let tile_class = move || {
        let c = cell.get();
        let mut classes = vec!["flap-tile"];
        if let CellContent::Color(color) = &c {
            classes.push("flap-tile--color");
            if is_light_color(color) {
                classes.push("flap-tile--light");
            }
        }
        if !has_received_update.get() {
            classes.push("flap-tile--loading");
        }
        if !is_animating.get() {
            classes.push("flap-tile--idle");
        }
        classes.join(" ")
    };

    let tile_style = move || {
        let mut style = format!("--col-index: {col_index};");
        if let CellContent::Color(color) = cell.get() {
            let css_color = color_to_css(&color);
            style.push_str(&format!(" --tile-bg: {css_color};"));
        }
        style
    };

    let half_bg_style = move || {
        if let CellContent::Color(color) = cell.get() {
            format!("background: {};", color_to_css(&color))
        } else {
            String::new()
        }
    };

    let old_char = move || cell_to_char(&prev_cell.get());
    let new_char = move || cell_to_char(&cell.get());

    view! {
        <div class=tile_class style=tile_style>
            // Static bottom layer: shows the NEW character underneath
            <div class="flap-bottom" style=half_bg_style>
                <span class="flap-char">{new_char}</span>
            </div>

            // Animated layers (only rendered during animation)
            {move || {
                if is_animating.get() {
                    let old_c = old_char();
                    let new_c = new_char();
                    let bg = half_bg_style();
                    Some(view! {
                        // Old top half flipping away
                        <div class="flap-top flap-top--flipping" style=bg.clone()>
                            <span class="flap-char">{old_c}</span>
                        </div>
                        // New bottom half flipping into place
                        <div class="flap-bottom-flip" style=bg>
                            <span class="flap-char">{new_c}</span>
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Static top layer: shows the NEW character (always visible)
            <div class="flap-top flap-top--static" style=half_bg_style>
                <span class="flap-char">{new_char}</span>
            </div>
        </div>
    }
}
