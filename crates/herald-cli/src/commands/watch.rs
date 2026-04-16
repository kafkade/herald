use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use crate::ui::BoardWidget;
use crate::ws_client::WsClient;

pub async fn run(server: String) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut board_rx) = WsClient::new(server);

    tokio::spawn(async move {
        client.run().await;
    });

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run_tui(&mut terminal, &mut board_rx).await;

    // Restore terminal — always runs even on error
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    board_rx: &mut tokio::sync::watch::Receiver<herald_common::BoardState>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initial draw with default state
    let board_state = board_rx.borrow().clone();
    terminal.draw(|frame| {
        frame.render_widget(BoardWidget::new(&board_state), frame.area());
    })?;

    loop {
        tokio::select! {
            result = board_rx.changed() => {
                if result.is_err() {
                    break; // Sender dropped
                }
                let board_state = board_rx.borrow().clone();
                terminal.draw(|frame| {
                    frame.render_widget(BoardWidget::new(&board_state), frame.area());
                })?;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                if event::poll(std::time::Duration::from_millis(0))? {
                    match event::read()? {
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                _ => {}
                            }
                        }
                        Event::Resize(_, _) => {
                            let board_state = board_rx.borrow().clone();
                            terminal.draw(|frame| {
                                frame.render_widget(BoardWidget::new(&board_state), frame.area());
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
