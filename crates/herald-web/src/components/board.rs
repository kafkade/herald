use super::tile::FlapTile;
use crate::ws::WebSocketState;
use herald_common::{BOARD_COLS, BOARD_ROWS};
use leptos::prelude::*;

/// The main board grid component.
/// Renders a 6×22 grid of FlapTile components using CSS Grid.
#[component]
pub fn Board(ws_state: WebSocketState) -> impl IntoView {
    view! {
        <div class="board-grid">
            {(0..BOARD_ROWS).map(|row| {
                let ws = ws_state.clone();
                (0..BOARD_COLS).map(move |col| {
                    let ws = ws.clone();
                    view! {
                        <FlapTile
                            cell=ws.grid[row][col]
                            prev_cell=ws.previous_grid[row][col]
                            col_index=col
                            update_counter=ws.update_counter
                            has_received_update=ws.has_received_update
                        />
                    }
                }).collect_view()
            }).collect_view()}
        </div>
    }
}
