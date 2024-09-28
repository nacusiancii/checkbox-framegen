use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Frame {
    IFrame { version: usize, timestamp: u64, data: Vec<usize> },
    PFrame { version: usize, timestamp: u64, changes: Vec<(usize, bool)> },
}

impl Frame {
    pub fn version(&self) -> usize {
        match self {
            Frame::IFrame { version, .. } => *version,
            Frame::PFrame { version, .. } => *version,
        }
    }
}

#[derive(Deserialize)]
pub struct ClientMessage {
    pub last_version: usize,
}
