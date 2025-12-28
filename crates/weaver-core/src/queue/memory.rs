//! In-memory queue implementation.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use super::{RetryPolicy, TaskRecord, TaskState};
use crate::domain::{TaskEnvelope, TaskId};
use crate::error::WeaverError;
use crate::observability::QueueCounts;
use crate::queue::{Queue, TaskLease};

/// Scheduled task entry for priority queue.
///
/// We use Reverse ordering so BinaryHeap acts as a min-heap (earliest first).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScheduledTask {
    next_run_at: Instant,
    task_id: TaskId,
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering: earlier times have higher priority
        other.next_run_at.cmp(&self.next_run_at)
    }
}

/// In-memory queue state.
struct InMemoryQueueState {
    /// All task records (single source of truth).
    records: HashMap<TaskId, TaskRecord>,

    /// Ready queue (TaskIds only).
    ready: VecDeque<TaskId>,

    /// Scheduled queue (retry backoff).
    scheduled: BinaryHeap<ScheduledTask>,

    /// Next task ID to assign.
    next_id: u64,

    /// Retry policy.
    retry_policy: RetryPolicy,
}

impl InMemoryQueueState {
    fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            records: HashMap::new(),
            ready: VecDeque::new(),
            scheduled: BinaryHeap::new(),
            next_id: 1,
            retry_policy,
        }
    }

    /// Allocate a new TaskId.
    fn allocate_id(&mut self) -> TaskId {
        let id = TaskId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /// Move tasks from scheduled to ready if their time has come.
    fn promote_scheduled_tasks(&mut self) {
        let now = Instant::now();
        while let Some(entry) = self.scheduled.peek() {
            if entry.next_run_at > now {
                break; // Heap is sorted, so we can stop
            }

            let entry = self.scheduled.pop().unwrap();
            if let Some(record) = self.records.get_mut(&entry.task_id) {
                if record.state == TaskState::RetryScheduled {
                    record.requeue();
                    self.ready.push_back(entry.task_id);
                }
            }
        }
    }

    /// Get counts by state for observability.
    fn counts_by_state(&self) -> QueueCounts {
        let mut counts = QueueCounts::default();
        for record in self.records.values() {
            match record.state {
                TaskState::Queued => counts.queued += 1,
                TaskState::Running => counts.running += 1,
                TaskState::Succeeded => counts.succeeded += 1,
                TaskState::RetryScheduled => counts.retry_scheduled += 1,
                TaskState::Dead => counts.dead += 1,
            }
        }
        counts
    }
}

/// In-memory queue implementation.
pub struct InMemoryQueue {
    state: Arc<Mutex<InMemoryQueueState>>,
    notify: Arc<Notify>,
}

impl InMemoryQueue {
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemoryQueueState::new(retry_policy))),
            notify: Arc::new(Notify::new()),
        }
    }
}

#[async_trait]
impl Queue for InMemoryQueue {
    async fn enqueue(&self, envelope: TaskEnvelope) -> Result<(), WeaverError> {
        let mut state = self.state.lock().await;
        let task_id = state.allocate_id();

        // Create new record (default: Queued, max_attempts from budget or default)
        let max_attempts = 5; // TODO: Get from envelope's task spec budget
        let record = TaskRecord::new(envelope, max_attempts);

        state.records.insert(task_id, record);
        state.ready.push_back(task_id);

        // Notify waiting workers
        drop(state);
        self.notify.notify_one();

        Ok(())
    }

    async fn lease(&self) -> Option<Box<dyn TaskLease>> {
        loop {
            let next_wake = {
                let mut state = self.state.lock().await;
                state.promote_scheduled_tasks();

                if let Some(task_id) = state.ready.pop_front() {
                    if let Some(record) = state.records.get_mut(&task_id) {
                        record.start_attempt();
                        let lease = InMemoryLease {
                            task_id,
                            envelope: record.envelope.clone(),
                            queue: Arc::clone(&self.state),
                            retry_policy: state.retry_policy.clone(),
                            notify: Arc::clone(&self.notify),
                        };
                        return Some(Box::new(lease));
                    }
                }

                // No ready tasks - check if we have scheduled tasks
                state.scheduled.peek().map(|entry| entry.next_run_at)
            };

            // Wait for notification OR next scheduled task time
            if let Some(wake_time) = next_wake {
                tokio::select! {
                    _ = self.notify.notified() => {},
                    _ = tokio::time::sleep_until(wake_time.into()) => {},
                }
            } else {
                self.notify.notified().await;
            }
        }
    }

    async fn counts_by_state(&self) -> Result<QueueCounts, WeaverError> {
        let state = self.state.lock().await;
        Ok(state.counts_by_state())
    }
}

/// Lease implementation for InMemoryQueue.
struct InMemoryLease {
    task_id: TaskId,
    envelope: TaskEnvelope,
    queue: Arc<Mutex<InMemoryQueueState>>,
    retry_policy: RetryPolicy,
    notify: Arc<Notify>,
}

#[async_trait]
impl TaskLease for InMemoryLease {
    fn envelope(&self) -> &TaskEnvelope {
        &self.envelope
    }

    async fn ack(self: Box<Self>) -> Result<(), WeaverError> {
        let mut state = self.queue.lock().await;
        if let Some(record) = state.records.get_mut(&self.task_id) {
            record.mark_succeeded();
        }
        Ok(())
    }

    async fn fail(self: Box<Self>, error: String) -> Result<(), WeaverError> {
        let should_notify = {
            let mut state = self.queue.lock().await;
            let Some(record) = state.records.get_mut(&self.task_id) else {
                return Ok(());
            };

            if record.attempts >= record.max_attempts {
                // Exhausted retries
                record.mark_dead(error);
                false // Terminal state, no need to notify
            } else {
                // Schedule retry with backoff
                let delay = self.retry_policy.next_delay(record.attempts);
                let next_run_at = Instant::now() + delay;
                record.schedule_retry(next_run_at, error);

                state.scheduled.push(ScheduledTask {
                    next_run_at,
                    task_id: self.task_id,
                });
                true // Scheduled task needs notification
            }
        }; // Lock released here

        // Notify outside the lock to avoid deadlock
        if should_notify {
            self.notify.notify_one();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskId, TaskType};

    #[tokio::test]
    async fn enqueue_and_counts() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(TaskId::new(999), TaskType::new("test"), serde_json::json!({}));

        queue.enqueue(env).await.unwrap();

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 1);
        assert_eq!(counts.running, 0);
    }

    #[tokio::test]
    async fn lease_transitions_to_running() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(TaskId::new(999), TaskType::new("test"), serde_json::json!({}));

        queue.enqueue(env).await.unwrap();

        let lease = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            queue.lease()
        ).await.unwrap().unwrap();

        assert_eq!(lease.envelope().task_type().as_str(), "test");

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 0);
        assert_eq!(counts.running, 1);
    }

    #[tokio::test]
    async fn ack_marks_succeeded() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(TaskId::new(999), TaskType::new("test"), serde_json::json!({}));

        queue.enqueue(env).await.unwrap();
        let lease = queue.lease().await.unwrap();
        lease.ack().await.unwrap();

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.succeeded, 1);
        assert_eq!(counts.running, 0);
    }
}
