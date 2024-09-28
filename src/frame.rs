use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Frame {
    IFrame { version: usize, timestamp: u64, data: Vec<usize> },
    PFrame { version: usize, timestamp: u64, changes: Vec<(usize, bool)> },
}

#[derive(Deserialize)]
pub struct ClientMessage {
    pub last_version: usize,
}
