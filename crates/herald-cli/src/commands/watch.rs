use crate::ws_client::WsClient;

pub async fn run(server: String) -> Result<(), Box<dyn std::error::Error>> {
    let (client, mut board_rx) = WsClient::new(server);

    // Spawn the WS client in a background task
    tokio::spawn(async move {
        client.run().await;
    });

    // Consume board state updates (placeholder for future TUI rendering)
    loop {
        if board_rx.changed().await.is_err() {
            break;
        }
        let state = board_rx.borrow().clone();
        eprintln!(
            "Board updated: current_item={:?}, timestamp={}",
            state.current_item, state.timestamp
        );
    }

    Ok(())
}
