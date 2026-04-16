use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    ExecutableCommand,
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use crate::ui::{BoardWidget, StatusBar};
use crate::ws_client::WsClient;

pub async fn run(server: String, fps: u16) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut board_rx, mut conn_rx) = WsClient::new(server.clone());

    tokio::spawn(async move {
        client.run().await;
    });

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    io::stdout().execute(cursor::Hide)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run_tui(&mut terminal, &mut board_rx, &mut conn_rx, &server, fps).await;

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
    server_url: &str,
    fps: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let tick_duration = Duration::from_millis(1000 / fps.max(1) as u64);
    let mut last_update: Option<Instant> = None;

    let draw = |frame: &mut ratatui::Frame,
                board_state: &herald_common::BoardState,
                conn_state: &crate::ws_client::ConnectionState,
                server_url: &str,
                last_update: Option<Instant>| {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        frame.render_widget(BoardWidget::new(board_state), chunks[0]);
        frame.render_widget(StatusBar::new(conn_state, server_url, last_update), chunks[1]);
    };

    // Initial draw
    let board_state = board_rx.borrow().clone();
    let conn_state = conn_rx.borrow().clone();
    terminal.draw(|frame| {
        draw(frame, &board_state, &conn_state, server_url, last_update);
    })?;

    loop {
        tokio::select! {
            result = board_rx.changed() => {
                if result.is_err() {
                    break;
                }
                last_update = Some(Instant::now());
                let board_state = board_rx.borrow().clone();
                let conn_state = conn_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &board_state, &conn_state, server_url, last_update);
                })?;
            }
            _ = conn_rx.changed() => {
                let board_state = board_rx.borrow().clone();
                let conn_state = conn_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &board_state, &conn_state, server_url, last_update);
                })?;
            }
            _ = tokio::time::sleep(tick_duration) => {
                let board_state = board_rx.borrow().clone();
                let conn_state = conn_rx.borrow().clone();
                terminal.draw(|frame| {
                    draw(frame, &board_state, &conn_state, server_url, last_update);
                })?;

                if event::poll(Duration::from_millis(0))? {
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
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
