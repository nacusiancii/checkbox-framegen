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
    pub fn new(state_size: usize, buffer_size: usize) -> Self {
        let state = BitVec::repeat(false, state_size);
        let (tx, _) = broadcast::channel(buffer_size);
        AppState {
            state,
            version: 0,
            tx,
            frame_buffer: VecDeque::with_capacity(buffer_size),
        }
    }

    pub fn update(&mut self, changes: Vec<(usize, bool)>) {
        self.version += 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut changed_indices = Vec::new();

        for (index, value) in changes {
            if self.state[index] != value {
                self.state.set(index, value);
                changed_indices.push((index, value));
            }
        }

        let frame = if self.version % crate::I_FRAME_INTERVAL == 0 {
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
        if self.frame_buffer.len() > crate::FRAME_BUFFER_SIZE {
            self.frame_buffer.pop_front();
        }

        let _ = self.tx.send(frame);
    }
}
