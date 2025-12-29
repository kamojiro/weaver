//! Task record: metadata + envelope.

use std::time::Instant;

use super::TaskState;
use crate::domain::{JobId, TaskEnvelope};

/// Metadata + envelope for a task in the queue.
///
/// Design:
/// - This is the "single source of truth" for task state.
/// - Queue structures (ready/scheduled) hold TaskId only.
/// - All state transitions happen here.
#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub envelope: TaskEnvelope,
    pub state: TaskState,

    pub job_id: Option<JobId>,

    /// Number of times this task has been executed (including current attempt if Running).
    pub attempts: u32,

    /// Maximum allowed attempts (from policy or budget).
    pub max_attempts: u32,

    /// Last error message (if any).
    pub last_error: Option<String>,

    /// When to retry next (for RetryScheduled state).
    pub next_run_at: Option<Instant>,

    /// Timestamps for observability.
    pub created_at: Instant,
    pub updated_at: Instant,
}

impl TaskRecord {
    pub fn new(envelope: TaskEnvelope, max_attempts: u32) -> Self {
        let now = Instant::now();
        Self {
            envelope,
            state: TaskState::Queued,
            job_id: None,
            attempts: 0,
            max_attempts,
            last_error: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new task record associated with a job.
    pub fn new_with_job(envelope: TaskEnvelope, max_attempts: u32, job_id: JobId) -> Self {
        let mut record = Self::new(envelope, max_attempts);
        record.job_id = Some(job_id);
        record
    }

    /// Mark as running (increment attempts).
    pub fn start_attempt(&mut self) {
        self.state = TaskState::Running;
        self.attempts += 1;
        self.updated_at = Instant::now();
    }

    /// Mark as succeeded.
    pub fn mark_succeeded(&mut self) {
        self.state = TaskState::Succeeded;
        self.updated_at = Instant::now();
    }

    /// Mark as dead (max attempts exceeded).
    pub fn mark_dead(&mut self, error: String) {
        self.state = TaskState::Dead;
        self.last_error = Some(error);
        self.updated_at = Instant::now();
    }

    /// Schedule retry with backoff.
    pub fn schedule_retry(&mut self, next_run_at: Instant, error: String) {
        self.state = TaskState::RetryScheduled;
        self.next_run_at = Some(next_run_at);
        self.last_error = Some(error);
        self.updated_at = Instant::now();
    }

    /// Move from RetryScheduled back to Queued.
    pub fn requeue(&mut self) {
        self.state = TaskState::Queued;
        self.next_run_at = None;
        self.updated_at = Instant::now();
    }
}
