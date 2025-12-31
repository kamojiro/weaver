//! In-memory queue implementation.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use super::{RetryPolicy, TaskRecord, TaskState};
use crate::domain::{
    Artifact, AttemptId, AttemptRecord, DecisionRecord, JobId, JobRecord, JobSpec, Outcome,
    TaskEnvelope, TaskId,
};
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
    /// All job records (single source of truth for jobs).
    jobs: HashMap<JobId, JobRecord>,

    /// All task records (single source of truth for tasks).
    records: HashMap<TaskId, TaskRecord>,

    /// Ready queue (TaskIds only).
    ready: VecDeque<TaskId>,

    /// AttemptRecords
    attempts: HashMap<AttemptId, AttemptRecord>,

    /// Decisions
    decisions: Vec<DecisionRecord>,

    /// Scheduled queue (retry backoff).
    scheduled: BinaryHeap<ScheduledTask>,

    /// Next job ID to assign.
    next_job_id: u64,

    /// Next task ID to assign.
    next_task_id: u64,

    /// Next attempt ID to assign.
    next_attempt_id: u64,

    /// Retry policy.
    retry_policy: RetryPolicy,
}

impl InMemoryQueueState {
    fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            jobs: HashMap::new(),
            records: HashMap::new(),
            ready: VecDeque::new(),
            attempts: HashMap::new(),
            decisions: Vec::new(),
            scheduled: BinaryHeap::new(),
            next_job_id: 1,
            next_task_id: 1,
            next_attempt_id: 1,
            retry_policy,
        }
    }

    /// Allocate a new JobId.
    fn allocate_job_id(&mut self) -> JobId {
        let id = JobId::new(self.next_job_id);
        self.next_job_id += 1;
        id
    }

    /// Allocate a new TaskId.
    fn allocate_task_id(&mut self) -> TaskId {
        let id = TaskId::new(self.next_task_id);
        self.next_task_id += 1;
        id
    }

    fn allocate_attempt_id(&mut self) -> AttemptId {
        let id = AttemptId::new(self.next_attempt_id);
        self.next_attempt_id += 1;
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
            if let Some(record) = self.records.get_mut(&entry.task_id)
                && record.state == TaskState::RetryScheduled
            {
                record.requeue();
                self.ready.push_back(entry.task_id);
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

    /// Create a new job.
    fn create_job(&mut self, spec: JobSpec) -> JobId {
        let id = self.allocate_job_id();
        let job_record = JobRecord::new(id, spec);
        self.jobs.insert(id, job_record);
        id
    }

    /// Get a job by ID.
    fn get_job(&self, job_id: JobId) -> Option<&JobRecord> {
        self.jobs.get(&job_id)
    }

    /// Get a mutable reference to a job by ID.
    fn get_job_mut(&mut self, job_id: JobId) -> Option<&mut JobRecord> {
        self.jobs.get_mut(&job_id)
    }

    /// Create a job with its tasks.
    fn create_job_with_tasks(&mut self, spec: JobSpec) -> JobId {
        let job_id = self.create_job(spec.clone());
        let max_attempts = spec.budget.max_attempts_per_task;
        for task_spec in &spec.tasks {
            let task_id = self.allocate_task_id();
            let task_type = task_spec
                .title
                .clone()
                .unwrap_or_else(|| "generic".to_string());
            let payload = serde_json::to_value(task_spec).unwrap_or(serde_json::json!({}));
            let envelope =
                TaskEnvelope::new(task_id, crate::domain::TaskType::new(task_type), payload);
            let task_record = TaskRecord::new_with_job(envelope, max_attempts, job_id);
            self.records.insert(task_id, task_record);
            self.ready.push_back(task_id);
            self.get_job_mut(job_id)
                .expect("job must exist after crate_job.")
                .add_task(task_id);
        }
        job_id
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
        let task_id = state.allocate_task_id();

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

                if let Some(task_id) = state.ready.pop_front()
                    && let Some(record) = state.records.get_mut(&task_id)
                {
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

impl InMemoryQueue {
    pub async fn submit_job(&self, spec: JobSpec) -> Result<JobId, WeaverError> {
        let job_id = {
            let mut state = self.state.lock().await;
            state.create_job_with_tasks(spec)
        };
        self.notify.notify_one();
        Ok(job_id)
    }

    /// Get attempt record by ID (for testing)
    #[cfg(test)]
    pub async fn get_attempt(&self, attempt_id: AttemptId) -> Option<AttemptRecord> {
        let state = self.state.lock().await;
        state.attempts.get(&attempt_id).cloned()
    }

    /// Get all decisions (for testing)
    #[cfg(test)]
    pub async fn get_decisions(&self) -> Vec<DecisionRecord> {
        let state = self.state.lock().await;
        state.decisions.clone()
    }

    /// Get all attempts (for testing)
    #[cfg(test)]
    pub async fn get_all_attempts(&self) -> Vec<AttemptRecord> {
        let state = self.state.lock().await;
        state.attempts.values().cloned().collect()
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

        // First, do all state operations (allocate, insert)
        let attempt_id = state.allocate_attempt_id();
        let attempt_record = AttemptRecord::new(
            attempt_id,
            self.task_id,
            self.envelope.payload().clone(),
            vec![],
            Outcome::success(),
        );
        state.attempts.insert(attempt_id, attempt_record);

        // Then, get mutable reference to record and update
        if let Some(record) = state.records.get_mut(&self.task_id) {
            record.mark_succeeded();
        }

        Ok(())
    }

    async fn fail(self: Box<Self>, error: String) -> Result<(), WeaverError> {
        let should_notify = {
            let mut state = self.queue.lock().await;
            let attempt_id = state.allocate_attempt_id();
            let attempt_record = AttemptRecord::new(
                attempt_id,
                self.task_id,
                self.envelope.payload().clone(),
                vec![Artifact::Stdout(error.clone())],
                Outcome::failure(error.clone()),
            );
            state.attempts.insert(attempt_id, attempt_record);

            let Some(record) = state.records.get_mut(&self.task_id) else {
                return Ok(());
            };

            if record.attempts >= record.max_attempts {
                let trigger = serde_json::json!({
                    "error": error,
                    "attempts": record.attempts,
                    "max_attempts": record.max_attempts,
                });
                let decision =
                    DecisionRecord::new(self.task_id, trigger, "retry_policy", "mark_dead", None);
                record.mark_dead(error);
                state.decisions.push(decision);
                false // Terminal state, no need to notify
            } else {
                // Schedule retry with backoff
                let delay = self.retry_policy.next_delay(record.attempts);
                let next_run_at = Instant::now() + delay;
 
                let trigger = serde_json::json!({
                    "error": error,
                    "attempts": record.attempts,
                    "max_attempts": record.max_attempts,
                });
                let context = Some(serde_json::json!({
                    "delay_secs": delay.as_secs(),
                    "next_run_at": format!("{:?}", next_run_at),
                }));

                let decision = DecisionRecord::new(
                    self.task_id,
                    trigger,
                    "retry_policy",
                    "schedule_retry",
                    context,
                );
                record.schedule_retry(next_run_at, error);
                state.decisions.push(decision);
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
    use core::task;
    use std::os::linux::raw::stat;

    use super::*;
    use crate::{domain::{OutcomeKind, TaskId, TaskType, decision}, queue};

    #[tokio::test]
    async fn enqueue_and_counts() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(
            TaskId::new(999),
            TaskType::new("test"),
            serde_json::json!({}),
        );

        queue.enqueue(env).await.unwrap();

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 1);
        assert_eq!(counts.running, 0);
    }

    #[tokio::test]
    async fn lease_transitions_to_running() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(
            TaskId::new(999),
            TaskType::new("test"),
            serde_json::json!({}),
        );

        queue.enqueue(env).await.unwrap();

        let lease = tokio::time::timeout(std::time::Duration::from_millis(100), queue.lease())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(lease.envelope().task_type().as_str(), "test");

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 0);
        assert_eq!(counts.running, 1);
    }

    #[tokio::test]
    async fn ack_marks_succeeded() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let env = TaskEnvelope::new(
            TaskId::new(999),
            TaskType::new("test"),
            serde_json::json!({}),
        );

        queue.enqueue(env).await.unwrap();
        let lease = queue.lease().await.unwrap();
        lease.ack().await.unwrap();

        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.succeeded, 1);
        assert_eq!(counts.running, 0);
    }

    // Phase 3 tests: AttemptRecord and DecisionRecord

    #[tokio::test]
    async fn test_attempt_record_is_saved_on_ack() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task = TaskEnvelope::new(
            TaskId::new(1001),  // Note: enqueue() will allocate a new task_id
            TaskType::new("test_task"),
            serde_json::json!({"key": "value"}),
        );
        queue.enqueue(task).await.unwrap();
        let lease = queue.lease().await.unwrap();
        let task_id = lease.envelope().task_id();  // Get the actual allocated task_id
        lease.ack().await.unwrap();
        let attempts = queue.get_all_attempts().await;
        assert_eq!(attempts.len(), 1);
        let attempt = &attempts[0];
        assert!(attempt.outcome.kind == OutcomeKind::Success);
        // Note: task_id in AttemptRecord is the allocated ID, not the TaskEnvelope's ID
        assert!(attempt.action == serde_json::json!({"key": "value"}));
        assert!(attempt.observation.is_empty());
    }

    #[tokio::test]
    async fn test_decision_record_is_saved_on_retry() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task = TaskEnvelope::new(
            TaskId::new(1001),
            TaskType::new("test_task"),
            serde_json::json!({"key": "value"}),
        );
        queue.enqueue(task).await.unwrap();
        let lease = queue.lease().await.unwrap();
        lease.fail("test error".to_string()).await.unwrap();
        let decisions = queue.get_decisions().await;
        assert_eq!(decisions.len(), 1);
        let decision = &decisions[0];
        assert_eq!(decision.policy, "retry_policy");
        assert_eq!(decision.decision, "schedule_retry");
        assert!(decision.trigger["error"] == "test error");
        let attempts = queue.get_all_attempts().await;
        assert_eq!(attempts.len(), 1);
        let attempt = &attempts[0];
        assert!(attempt.outcome.kind == OutcomeKind::Failure);
        assert!(attempt.observation.len() == 1);
        match &attempt.observation[0] {
            Artifact::Stdout(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Artifact::Stdout"), 
        }


    }

    #[tokio::test]
    async fn test_decision_record_is_saved_on_mark_dead() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task_id = TaskId::new(1001);
        {
            let mut state = queue.state.lock().await;
            let max_attempts = 1;
            let envelope = TaskEnvelope::new(
                task_id,
                TaskType::new("test_task"),
                serde_json::json!({"key": "value"}),
            );
            let record = TaskRecord::new(envelope, max_attempts);
            state.records.insert(task_id, record);
            state.ready.push_back(task_id);
        }
        queue.notify.notify_one();
        let lease = queue.lease().await.unwrap();
        lease.fail("err1".to_string()).await.unwrap();

        let decisions = queue.get_decisions().await;
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "mark_dead");
        assert_eq!(decisions[0].policy, "retry_policy");
        assert_eq!(decisions[0].trigger["attempts"], 1);
        assert_eq!(decisions[0].trigger["max_attempts"], 1);



    }
}
