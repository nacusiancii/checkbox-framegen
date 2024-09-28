use bitvec::prelude::*;
use std::collections::VecDeque;
use tokio::sync::broadcast;
use crate::frame::Frame;

pub struct AppState {
    pub state: BitVec,
    pub version: usize,
    pub tx: broadcast::Sender<Frame>,
    pub frame_buffer: VecDeque<Frame>,
}

impl AppState {
    pub fn new(initial_state: BitVec, buffer_size: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer_size);
        let mut app_state = AppState {
            state: initial_state,
            version: 0,
            tx,
            frame_buffer: VecDeque::with_capacity(buffer_size),
        };
        
        // Create and store initial I-frame
        let initial_frame = Frame::IFrame {
            version: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            data: app_state.state.clone().into_vec(),
        };
        app_state.frame_buffer.push_back(initial_frame);
        
        app_state
    }

    pub fn update(&mut self, changes: Vec<(usize, bool)>) {
        self.version += 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut changed_indices = Vec::new();
        for (index, value) in changes {
            if index < self.state.len() && self.state[index] != value {
                self.state.set(index, value);
                changed_indices.push((index, value));
            }
        }

        let frame = if self.version % I_FRAME_INTERVAL == 0 {
            Frame::IFrame {
                version: self.version,
                timestamp,
                data: self.state.clone().into_vec(),
            }
        } else {
            Frame::PFrame {
                version: self.version,
                timestamp,
                changes: changed_indices,
            }
        };

        self.frame_buffer.push_back(frame.clone());
        if self.frame_buffer.len() > FRAME_BUFFER_SIZE {
            self.frame_buffer.pop_front();
        }

        let _ = self.tx.send(frame);
    }
}
