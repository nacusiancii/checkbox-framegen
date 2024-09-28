use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

mod state;
mod frame;
mod websocket;

use state::AppState;
use frame::Frame;
use websocket::ws_handler;

const STATE_SIZE: usize = 1_000_000;
const I_FRAME_INTERVAL: usize = 2000; // Send I-frame every 2000 versions
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

    println!("Server running on http://localhost:3000");
    axum::Server::bind(&"0.0.0.0:8081".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
