//! Queue module: state management, retry logic, and in-memory implementation.

mod memory;
mod record;
mod retry;
mod state;

pub use memory::InMemoryQueue;
pub use record::TaskRecord;
pub use retry::RetryPolicy;
pub use state::TaskState;

use async_trait::async_trait;

use crate::domain::{Decision, Outcome, TaskEnvelope};
use crate::error::WeaverError;

/// A leased task for processing.
///
/// Design intent:
/// - Queue manages state transitions (Queued -> Running -> ...).
/// - Worker/Runtime executes side effects and reports the result.
/// - `TaskEnvelope` is exposed as an immutable reference to avoid accidental mutation.
///
/// Phase 4-1 adds Outcome + Decision based completion:
/// - `get_task_record()`: Get fresh TaskRecord for Decider
/// - `complete()`: Complete with Outcome and Decision
/// - `ack()`/`fail()`: Deprecated, kept for transition
#[async_trait]
pub trait TaskLease: Send {
    fn envelope(&self) -> &TaskEnvelope;

    /// Get fresh TaskRecord for decision-making.
    ///
    /// Phase 4-1: Worker needs TaskRecord to call Decider.
    /// This method re-acquires TaskRecord from queue state to prevent stale data.
    async fn get_task_record(&self) -> Result<TaskRecord, WeaverError>;

    /// Complete task execution with Outcome and Decision.
    ///
    /// Phase 4-1: New completion method that takes explicit Decision.
    /// This replaces the retry logic that was embedded in fail().
    ///
    /// # Arguments
    /// * `outcome` - The outcome from handler execution
    /// * `decision` - The decision from Decider (Retry or MarkDead)
    async fn complete(
        self: Box<Self>,
        outcome: Outcome,
        decision: Decision,
    ) -> Result<(), WeaverError>;

    /// Mark success.
    ///
    /// Phase 4-1: Still used for SUCCESS outcomes (bypasses Decider).
    async fn ack(self: Box<Self>) -> Result<(), WeaverError>;

    /// Mark failure (queue decides retry/dead policy).
    ///
    /// **Deprecated in Phase 4-1**: Use `complete()` instead.
    /// Kept temporarily for transition period.
    #[deprecated(note = "Use complete() with Decider instead")]
    async fn fail(self: Box<Self>, error: String) -> Result<(), WeaverError>;
}

/// Queue port (interface).
/// v1 is in-memory, but this trait is the seam for swapping implementations later.
#[async_trait]
pub trait Queue: Send + Sync {
    /// Enqueue a new task.
    async fn enqueue(&self, envelope: TaskEnvelope) -> Result<(), WeaverError>;

    /// Lease one ready task (waits until available, or returns None if shutdown).
    async fn lease(&self) -> Option<Box<dyn TaskLease>>;

    /// Observability hook (optional but useful).
    async fn counts_by_state(&self) -> Result<crate::observability::QueueCounts, WeaverError>;
}
