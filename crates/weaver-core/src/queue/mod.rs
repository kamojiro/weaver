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

use crate::domain::TaskEnvelope;
use crate::error::WeaverError;

/// A leased task for processing.
/// The worker owns this lease and must either `ack` or `fail`.
///
/// Design intent:
/// - Queue manages state transitions (Queued -> Running -> ...).
/// - Worker/Runtime executes side effects and reports the result.
/// - `TaskEnvelope` is exposed as an immutable reference to avoid accidental mutation.
#[async_trait]
pub trait TaskLease: Send {
    fn envelope(&self) -> &TaskEnvelope;

    /// Mark success.
    async fn ack(self: Box<Self>) -> Result<(), WeaverError>;

    /// Mark failure (queue decides retry/dead policy).
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
