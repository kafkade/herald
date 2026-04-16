use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    ExecutableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use crate::ui::animation::BoardAnimation;
use crate::ui::{BoardWidget, DisplayGrid, StatusBar};
use crate::ws_client::WsClient;

/// Default duration for each character step in the cycling animation.
const STEP_DURATION: Duration = Duration::from_millis(50);

/// Default cascade delay between columns.
const STAGGER_PER_COLUMN: Duration = Duration::from_millis(20);

/// Minimum tick interval during animation for smooth rendering.
const ANIMATION_TICK: Duration = Duration::from_millis(20);

pub async fn run(server: String, fps: u16) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut board_rx, mut conn_rx, mut queue_rx) = WsClient::new(server.clone());

    tokio::spawn(async move {
        client.run().await;
    });

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run_tui(
        &mut terminal,
        &mut board_rx,
        &mut conn_rx,
        &mut queue_rx,
        &server,
        fps,
    )
    .await;

    // Restore terminal — always runs even on error
    disable_raw_mode()?;
    io::stdout().execute(cursor::Show)?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    board_rx: &mut tokio::sync::watch::Receiver<herald_common::BoardState>,
    conn_rx: &mut tokio::sync::watch::Receiver<crate::ws_client::ConnectionState>,
    queue_rx: &mut tokio::sync::watch::Receiver<crate::ws_client::QueueInfoState>,
    server_url: &str,
    fps: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let normal_tick = Duration::from_millis(1000 / fps.max(1) as u64);
    let mut last_update: Option<Instant> = None;

    // Animation state
    let mut current_display = DisplayGrid::from_board_state(&board_rx.borrow());
    let mut animation: Option<BoardAnimation> = None;

    let draw = |frame: &mut ratatui::Frame,
                display: &DisplayGrid,
                conn_state: &crate::ws_client::ConnectionState,
                queue_info: &crate::ws_client::QueueInfoState,
                server_url: &str,
                last_update: Option<Instant>| {
        let area = frame.area();
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        frame.render_widget(BoardWidget::new(display), chunks[0]);
        frame.render_widget(
            StatusBar::new(conn_state, server_url, last_update, queue_info),
            chunks[1],
        );
    };

    // Initial draw
    let conn_state = conn_rx.borrow().clone();
    let queue_info = queue_rx.borrow().clone();
    terminal.draw(|frame| {
        draw(
            frame,
            &current_display,
            &conn_state,
            &queue_info,
            server_url,
            last_update,
        );
    })?;

    loop {
        // Use a shorter tick when animating for smooth rendering
        let tick_duration = if animation.is_some() {
            ANIMATION_TICK.min(normal_tick)
        } else {
            normal_tick
        };

        tokio::select! {
            result = board_rx.changed() => {
                if result.is_err() {
                    break;
                }
                last_update = Some(Instant::now());
                let new_board = board_rx.borrow().clone();

                // Start animation from the currently visible display state
                let from_display = if let Some(ref anim) = animation {
                    anim.sample(Instant::now())
                } else {
                    current_display.clone()
                };

                let new_anim = BoardAnimation::new(
                    &from_display,
                    &new_board,
                    STEP_DURATION,
                    STAGGER_PER_COLUMN,
                );

                if new_anim.has_changes() {
                    animation = Some(new_anim);

                    // Render immediately with the new animation's first frame
                    let now = Instant::now();
                    current_display = animation.as_ref().unwrap().sample(now);
                } else {
                    // No changes — update display directly, skip animation
                    current_display = DisplayGrid::from_board_state(&new_board);
                }

                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = conn_rx.changed() => {
                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = queue_rx.changed() => {
                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = tokio::time::sleep(tick_duration) => {
                // Advance animation if active
                if let Some(ref anim) = animation {
                    let now = Instant::now();
                    current_display = anim.sample(now);

                    if anim.is_complete(now) {
                        // Animation finished — settle on the final target state
                        animation = None;
                    }
                }

                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &conn_state, &queue_info, server_url, last_update);
                })?;

                while event::poll(Duration::from_millis(0))? {
                    match event::read()? {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    break
                                }
                                _ => {}
                            }
                        }
                        Event::Resize(_width, _height) => {
                            // Cancel any in-progress animation and snap to target
                            if let Some(ref anim) = animation {
                                current_display = DisplayGrid::from_board_state(anim.target());
                                animation = None;
                            }
                            // Re-draw immediately with the new terminal size
                            let conn_state = conn_rx.borrow().clone();
                            let queue_info = queue_rx.borrow().clone();
                            terminal.draw(|frame| {
                                draw(frame, &current_display, &conn_state, &queue_info, server_url, last_update);
                            })?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
