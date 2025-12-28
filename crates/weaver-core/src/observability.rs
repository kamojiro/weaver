use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueCounts {
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub retry_scheduled: usize,
    pub dead: usize,
}
