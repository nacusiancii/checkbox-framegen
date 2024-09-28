use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::state::AppState;
use crate::frame::{Frame, ClientMessage};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<AppState>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<Mutex<AppState>>) {
    // Send initial state
    send_initial_state(&mut socket, &state).await;

    let mut rx = state.lock().await.tx.subscribe();

    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        handle_client_message(&mut socket, &state, client_msg).await;
                    }
                } else if msg.is_err() {
                    break;
                }
            }
            Ok(frame) = rx.recv() => {
                if socket.send(Message::Binary(bincode::serialize(&frame).unwrap())).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
}

async fn send_initial_state(socket: &mut WebSocket, state: &Arc<Mutex<AppState>>) {
    let state_guard = state.lock().await;
    let initial_frame = Frame::IFrame {
        version: state_guard.version,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        data: state_guard.state.clone().into_vec(),
    };
    drop(state_guard);
    let _ = socket
        .send(Message::Binary(bincode::serialize(&initial_frame).unwrap()))
        .await;
}

async fn handle_client_message(socket: &mut WebSocket, state: &Arc<Mutex<AppState>>, msg: ClientMessage) {
    let state_guard = state.lock().await;
    if msg.last_version < state_guard.version {
        if state_guard.version - msg.last_version > crate::MAX_CATCHUP_FRAMES {
            // Too many missed frames, send full state
            drop(state_guard);
            send_initial_state(socket, state).await;
        } else {
            // Send missed frames
            for frame in state_guard.frame_buffer.iter().filter(|f| f.version() > msg.last_version) {
                let _ = socket
                    .send(Message::Binary(bincode::serialize(frame).unwrap()))
                    .await;
            }
        }
    }
}
