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

/// Minimum tick interval during animation for smooth rendering.
const ANIMATION_TICK: Duration = Duration::from_millis(20);

/// Animation speed presets for split-flap flip animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum AnimationSpeed {
    /// Fast: 25ms/step, 10ms stagger
    Fast,
    /// Normal: 50ms/step, 20ms stagger (default)
    Normal,
    /// Slow: 100ms/step, 40ms stagger
    Slow,
    /// Off: instant transitions, no animation
    Off,
}

impl AnimationSpeed {
    pub fn step_duration(&self) -> Duration {
        match self {
            Self::Fast => Duration::from_millis(25),
            Self::Normal => Duration::from_millis(50),
            Self::Slow => Duration::from_millis(100),
            Self::Off => Duration::ZERO,
        }
    }

    pub fn stagger_per_column(&self) -> Duration {
        match self {
            Self::Fast => Duration::from_millis(10),
            Self::Normal => Duration::from_millis(20),
            Self::Slow => Duration::from_millis(40),
            Self::Off => Duration::ZERO,
        }
    }
}

pub async fn run(
    server: String,
    fps: u16,
    speed: AnimationSpeed,
) -> Result<(), Box<dyn std::error::Error>> {
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
        speed,
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
    speed: AnimationSpeed,
) -> Result<(), Box<dyn std::error::Error>> {
    let normal_tick = Duration::from_millis(1000 / fps.max(1) as u64);
    let mut last_update: Option<Instant> = None;

    // Animation state — pre-allocate the display buffer once and reuse it
    let mut current_display = DisplayGrid::from_board_state(&board_rx.borrow());
    let mut animation: Option<BoardAnimation> = None;
    let mut current_theme = board_rx.borrow().theme.clone();

    // Frame skip: track when we last rendered to avoid catching up
    let mut last_frame_time = Instant::now();

    let draw = |frame: &mut ratatui::Frame,
                display: &DisplayGrid,
                theme: &herald_common::ThemeKind,
                conn_state: &crate::ws_client::ConnectionState,
                queue_info: &crate::ws_client::QueueInfoState,
                server_url: &str,
                last_update: Option<Instant>| {
        let area = frame.area();
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        frame.render_widget(BoardWidget::new(display, theme), chunks[0]);
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
            &current_theme,
            &conn_state,
            &queue_info,
            server_url,
            last_update,
        );
    })?;

    let mut should_quit = false;

    loop {
        if should_quit {
            break;
        }

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
                current_theme = new_board.theme.clone();

                if speed == AnimationSpeed::Off {
                    // Instant transition — skip animation entirely
                    animation = None;
                    current_display.fill_from_board_state(&new_board);
                } else {
                    // Start animation from the currently visible display state
                    if let Some(ref anim) = animation {
                        // Reuse current_display buffer for the mid-animation snapshot
                        anim.sample_into(Instant::now(), &mut current_display);
                    }

                    let new_anim = BoardAnimation::with_options(
                        &current_display,
                        &new_board,
                        speed.step_duration(),
                        speed.stagger_per_column(),
                        queue_rx.borrow().is_countdown_active,
                    );

                    if new_anim.has_changes() {
                        animation = Some(new_anim);

                        // Render immediately with the new animation's first frame
                        let now = Instant::now();
                        animation.as_ref().unwrap().sample_into(now, &mut current_display);
                    } else {
                        // No changes — update display directly, skip animation
                        current_display.fill_from_board_state(&new_board);
                    }
                }

                last_frame_time = Instant::now();

                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &current_theme, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = conn_rx.changed() => {
                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &current_theme, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = queue_rx.changed() => {
                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &current_theme, &conn_state, &queue_info, server_url, last_update);
                })?;
            }
            _ = tokio::time::sleep(tick_duration) => {
                // Advance animation if active
                if let Some(ref anim) = animation {
                    let now = Instant::now();
                    let elapsed_since_last_frame = now.duration_since(last_frame_time);

                    if elapsed_since_last_frame >= tick_duration {
                        // Frame skip: if more than 3 ticks behind we still just
                        // sample the latest state (no intermediate catch-up).
                        anim.sample_into(now, &mut current_display);
                        last_frame_time = now;
                    }

                    if anim.is_complete(now) {
                        // Animation finished — settle on the final target state
                        animation = None;
                    }
                }

                let conn_state = conn_rx.borrow().clone();
                let queue_info = queue_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &current_display, &current_theme, &conn_state, &queue_info, server_url, last_update);
                })?;

                while event::poll(Duration::from_millis(0))? {
                    match event::read()? {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    should_quit = true;
                                    break;
                                }
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    should_quit = true;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Event::Resize(_width, _height) => {
                            // Cancel any in-progress animation and snap to target
                            if let Some(ref anim) = animation {
                                current_display.fill_from_board_state(anim.target());
                                animation = None;
                            }
                            // Re-draw immediately with the new terminal size
                            let conn_state = conn_rx.borrow().clone();
                            let queue_info = queue_rx.borrow().clone();
                            terminal.draw(|frame| {
                                draw(frame, &current_display, &current_theme, &conn_state, &queue_info, server_url, last_update);
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
