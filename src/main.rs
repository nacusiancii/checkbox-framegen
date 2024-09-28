use axum::{
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

mod state;
mod frame;
mod websocket;

use state::AppState;
use websocket::ws_handler;

const STATE_SIZE: usize = 1_000_000;
const I_FRAME_INTERVAL: usize = 100; // Send I-frame every 100 versions
const FRAME_BUFFER_SIZE: usize = 1000; // Store last 1000 frames
const MAX_CATCHUP_FRAMES: usize = 500; // Maximum number of P-frames to send for catch-up

#[tokio::main]
async fn main() {
    let state = AppState::new(STATE_SIZE, FRAME_BUFFER_SIZE);
    let (tx, _) = broadcast::channel(100);

    let app_state = Arc::new(Mutex::new(state));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    println!("Server running on http://localhost:8081");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
