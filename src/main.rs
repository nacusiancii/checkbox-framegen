use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use reqwest::Client;
use bitvec::prelude::BitVec;
mod state;
mod frame;
mod websocket;
use state::AppState;
use websocket::ws_handler;

const STATE_SIZE: usize = 1_000_000;
const I_FRAME_INTERVAL: usize = 100;
const FRAME_BUFFER_SIZE: usize = 1000;
const MAX_CATCHUP_FRAMES: usize = 500;
const UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100); // 10 times per second

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let initial_state = fetch_initial_state(&client).await?;
    let state = AppState::new(initial_state, FRAME_BUFFER_SIZE);
    let app_state = Arc::new(Mutex::new(state));
    
    // Spawn a task to update the state
    let update_state = app_state.clone();
    tokio::spawn(async move {
        let client = Client::new();
        loop {
            if let Err(e) = update_state_from_snapshot(&client, update_state.clone()).await {
                eprintln!("Error updating state: {:?}", e);
            }
            tokio::time::sleep(UPDATE_INTERVAL).await;
        }
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    println!("Server running on http://localhost:8081");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn fetch_initial_state(client: &Client) -> Result<BitVec, Box<dyn std::error::Error>> {
    let response = client.get("http://localhost:8080/snapshot").send().await?;
    let snapshot: SnapshotResponse = response.json().await?;
    
    let mut state = BitVec::repeat(false, STATE_SIZE);
    for &index in &snapshot.data {
        if index < STATE_SIZE {
            state.set(index, true);
        }
    }
    Ok(state)
}

async fn update_state_from_snapshot(client: &Client, state: Arc<Mutex<AppState>>) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get("http://localhost:8080/snapshot").send().await?;
    let snapshot: SnapshotResponse = response.json().await?;
    
    let mut state_guard = state.lock().await;
    let changes: Vec<(usize, bool)> = snapshot.data.into_iter()
        .filter(|&index| index < STATE_SIZE)
        .map(|index| (index, true))
        .collect();
    
    state_guard.update(changes);
    Ok(())
}

#[derive(serde::Deserialize)]
struct SnapshotResponse {
    data: Vec<usize>,
}
