//! Task state machine for the queue.

use serde::{Deserialize, Serialize};

/// Task state (v1 minimal set).
///
/// State transitions:
/// - Queued -> Running -> Succeeded
/// - Queued -> Running -> RetryScheduled -> Queued (loop until max_attempts)
/// - Queued -> Running -> Dead (when max_attempts exceeded)
/// - Queued -> Running -> Decomposed (when task is decomposed into child tasks)
///
/// Design note: Using an enum ensures exhaustive matching and prevents invalid states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    /// Ready to run immediately.
    Queued,

    /// Currently being executed by a worker.
    Running,

    /// Successfully completed.
    Succeeded,

    /// Waiting for retry (delayed due to backoff).
    RetryScheduled,

    /// Failed permanently (max_attempts exceeded).
    Dead,

    /// Decomposed into child tasks (task completed its role).
    Decomposed,
}

impl TaskState {
    /// Is this a terminal state (no further transitions)?
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            TaskState::Succeeded | TaskState::Decomposed | TaskState::Dead
        )
    }

    /// Is this task runnable (eligible for lease)?
    pub fn is_runnable(self) -> bool {
        matches!(self, TaskState::Queued)
    }
}
