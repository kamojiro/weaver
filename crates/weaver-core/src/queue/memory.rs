//! In-memory queue implementation.

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use super::{DependencyGraph, RetryPolicy, TaskRecord, TaskState};
use crate::domain::{
    Artifact, AttemptId, AttemptRecord, Decision, DecisionRecord, JobId, JobRecord, JobResult,
    JobSpec, JobStateView, JobStatus, Outcome, TaskEnvelope, TaskId, TaskSpec,
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

    /// Dependency graph for task dependencies.
    dependency_graph: DependencyGraph,

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
            dependency_graph: DependencyGraph::new(),
            next_job_id: 1,
            next_task_id: 1,
            next_attempt_id: 1,
            retry_policy,
        }
    }

    /// Allocate a new JobId.
    fn allocate_job_id(&mut self) -> JobId {
        let id = JobId::new(self.next_job_id as u128);
        self.next_job_id += 1;
        id
    }

    /// Allocate a new TaskId.
    fn allocate_task_id(&mut self) -> TaskId {
        let id = TaskId::new(self.next_task_id as u128);
        self.next_task_id += 1;
        id
    }

    fn allocate_attempt_id(&mut self) -> AttemptId {
        let id = AttemptId::new(self.next_attempt_id as u128);
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
                TaskState::Decomposed => counts.decomposed += 1,
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
            let envelope =
                TaskEnvelope::new(task_id, task_spec.task_type.clone(), task_spec.payload.clone());
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
    pub(crate) state: Arc<Mutex<InMemoryQueueState>>,
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

                if let Some(task_id) = state.ready.pop_front() {
                    // Phase 6/7: Check job state before leasing
                    // First, get job_id from record (immutable borrow)
                    let job_id = state.records.get(&task_id).and_then(|r| r.job_id);

                    // Check job state if task belongs to a job
                    if let Some(job_id) = job_id {
                        if let Some(job) = state.get_job_mut(job_id) {
                            // Phase 6: Check deadline
                            if job.is_deadline_exceeded() {
                                job.mark_stuck();
                                // Skip this task and continue to next iteration
                                continue;
                            }

                            // Phase 7.2: Skip tasks from cancelled jobs
                            if job.state == crate::domain::JobState::Cancelled {
                                // Skip this task and continue to next iteration
                                continue;
                            }
                        }
                    }

                    // Job state OK, start task attempt
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

impl InMemoryQueue {
    pub async fn submit_job(&self, spec: JobSpec) -> Result<JobId, WeaverError> {
        let job_id = {
            let mut state = self.state.lock().await;
            state.create_job_with_tasks(spec)
        };
        self.notify.notify_one();
        Ok(job_id)
    }

    /// Get job status by ID (Phase 7.1).
    pub async fn get_status(&self, job_id: JobId) -> Result<JobStatus, WeaverError> {
        let state = self.state.lock().await;

        let job = state
            .get_job(job_id)
            .ok_or_else(|| WeaverError::Other(format!("Job {} not found", job_id)))?;

        // Count tasks by state
        let mut completed_tasks = 0;
        let mut failed_tasks = 0;
        let mut running_tasks = 0;

        for task_id in &job.task_ids {
            if let Some(record) = state.records.get(task_id) {
                match record.state {
                    TaskState::Succeeded => completed_tasks += 1,
                    TaskState::Dead => failed_tasks += 1,
                    TaskState::Running | TaskState::Queued | TaskState::RetryScheduled => {
                        running_tasks += 1
                    }
                    TaskState::Decomposed => {} // Don't count decomposed tasks
                }
            }
        }

        // Convert Instant to milliseconds (elapsed since job creation)
        let created_at_ms = job.created_at.elapsed().as_millis() as u64;
        let updated_at_ms = job.updated_at.elapsed().as_millis() as u64;
        let deadline_at_ms = job
            .deadline_at
            .map(|deadline| deadline.elapsed().as_millis() as u64);

        Ok(JobStatus {
            job_id,
            state: JobStateView::from(job.state),
            created_at_ms,
            updated_at_ms,
            deadline_at_ms,
            total_tasks: job.task_ids.len(),
            completed_tasks,
            failed_tasks,
            running_tasks,
        })
    }

    /// Cancel a job by ID (Phase 7.2).
    ///
    /// v1: Simply marks the job as cancelled. Running tasks will continue
    /// but new tasks from this job won't be leased.
    pub async fn cancel_job(&self, job_id: JobId) -> Result<(), WeaverError> {
        let mut state = self.state.lock().await;

        let job = state
            .get_job_mut(job_id)
            .ok_or_else(|| WeaverError::Other(format!("Job {} not found", job_id)))?;

        job.mark_cancelled();
        Ok(())
    }

    /// Get job result with full execution history (Phase 7.3).
    pub async fn get_result(&self, job_id: JobId) -> Result<JobResult, WeaverError> {
        let state = self.state.lock().await;

        let job = state
            .get_job(job_id)
            .ok_or_else(|| WeaverError::Other(format!("Job {} not found", job_id)))?;

        // Collect all attempts for tasks in this job
        let mut attempts = Vec::new();
        for attempt_record in state.attempts.values() {
            if job.task_ids.contains(&attempt_record.task_id) {
                attempts.push(attempt_record.clone());
            }
        }

        // Collect all decisions for tasks in this job
        let mut decisions = Vec::new();
        for decision_record in &state.decisions {
            if job.task_ids.contains(&decision_record.task_id) {
                decisions.push(decision_record.clone());
            }
        }

        // Convert timestamps
        let created_at_ms = job.created_at.elapsed().as_millis() as u64;
        let updated_at_ms = job.updated_at.elapsed().as_millis() as u64;
        let deadline_at_ms = job
            .deadline_at
            .map(|deadline| deadline.elapsed().as_millis() as u64);

        Ok(JobResult {
            job_id,
            state: JobStateView::from(job.state),
            created_at_ms,
            updated_at_ms,
            deadline_at_ms,
            task_ids: job.task_ids.clone(),
            attempts,
            decisions,
        })
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

    async fn get_task_record(&self) -> Result<TaskRecord, WeaverError> {
        let state = self.queue.lock().await;
        state
            .records
            .get(&self.task_id)
            .cloned()
            .ok_or_else(|| WeaverError::Other("task record not found".into()))
    }

    async fn complete(
        self: Box<Self>,
        outcome: Outcome,
        decision: Decision,
    ) -> Result<(), WeaverError> {
        let attempt_record = {
            let mut state = self.queue.lock().await;

            // First, do all state operations (allocate, insert)
            let attempt_id = state.allocate_attempt_id();
            let attempt_record = AttemptRecord::new(
                attempt_id,
                self.task_id,
                self.envelope.payload().clone(),
                outcome.artifacts.clone(),
                outcome.clone(),
            );
            state.attempts.insert(attempt_id, attempt_record.clone());
            attempt_record
        };

        let should_notify = match decision {
            Decision::Retry { delay, reason } => {
                let next_run_at = Instant::now() + delay;
                let decision_record = DecisionRecord::new(
                    self.task_id,
                    serde_json::json!({
                        "attempt_id": attempt_record.attempt_id,
                        "outcome": format!("{:?}", outcome.kind),
                    }),
                    "retry_policy".to_string(),
                    "schedule_retry".to_string(),
                    Some(serde_json::json!({
                        "delay_secs": delay.as_secs(),
                        "next_run_at": format!("{:?}", next_run_at),
                    })),
                );
                let mut state = self.queue.lock().await;
                if let Some(record) = state.records.get_mut(&self.task_id) {
                    record.schedule_retry(next_run_at, reason);
                    state.decisions.push(decision_record);
                    state.scheduled.push(ScheduledTask {
                        next_run_at,
                        task_id: self.task_id,
                    });
                }
                true
            }
            Decision::MarkDead { reason } => {
                let decision_record = DecisionRecord::new(
                    self.task_id,
                    serde_json::json!({
                        "attempt_id": attempt_record.attempt_id,
                        "outcome": format!("{:?}", outcome.kind),
                    }),
                    "retry_policy".to_string(),
                    "mark_dead".to_string(),
                    None,
                );
                let mut state = self.queue.lock().await;
                if let Some(record) = state.records.get_mut(&self.task_id) {
                    record.mark_dead(reason);
                    state.decisions.push(decision_record);
                };
                false
            }
            Decision::Decompose {
                child_tasks,
                reason,
            } => {
                let child_ids = self.add_child_tasks(child_tasks).await?;
                let decision_record = DecisionRecord::new(
                    self.task_id,
                    serde_json::json!({
                        "attempt_id": attempt_record.attempt_id,
                        "outcome": format!("{:?}", outcome.kind),
                        "child_task_ids": child_ids.iter().map(|id| id.as_u64()).collect::<Vec<u64>>(),
                    }),
                    "decomposition".to_string(),
                    "decompose".to_string(),
                    Some(serde_json::json!({
                        "reason": reason,
                    })),
                );
                let mut state = self.queue.lock().await;

                if let Some(record) = state.records.get_mut(&self.task_id) {
                    record.state = TaskState::Decomposed;
                    state.decisions.push(decision_record);
                }
                false
            }
        };

        if should_notify {
            self.notify.notify_one();
        }
        Ok(())
    }

    async fn add_child_tasks(
        &self,
        child_specs: Vec<TaskSpec>,
    ) -> Result<Vec<TaskId>, WeaverError> {
        // Phase 1: Acquire lock, get parent info, allocate TaskIds
        let (parent_job_id, max_attempts, task_ids) = {
            let mut state = self.queue.lock().await;

            let parent = state
                .records
                .get(&self.task_id)
                .ok_or_else(|| WeaverError::Other("parent task not found".into()))?;

            let parent_job_id = parent
                .job_id
                .ok_or_else(|| WeaverError::Other("parent task has no associated job".into()))?;

            let max_attempts = parent.max_attempts;

            // Pre-allocate all TaskIds while holding the lock
            let task_ids: Vec<TaskId> = (0..child_specs.len())
                .map(|_| state.allocate_task_id())
                .collect();

            (parent_job_id, max_attempts, task_ids)
        }; // Lock is released here

        // Phase 2: Create TaskRecords outside the lock (no I/O, but reduces lock contention)
        let task_records: Vec<(TaskId, TaskRecord)> = child_specs
            .into_iter()
            .zip(task_ids.iter())
            .map(|(spec, &task_id)| {
                let envelope = TaskEnvelope::new(task_id, spec.task_type, spec.payload);
                let record =
                    TaskRecord::new_child(envelope, max_attempts, parent_job_id, self.task_id);
                (task_id, record)
            })
            .collect();

        // Phase 3: Re-acquire lock and insert all records
        {
            let mut state = self.queue.lock().await;

            for (task_id, record) in task_records {
                state.records.insert(task_id, record);
                state.ready.push_back(task_id);
            }

            // Update parent's child_task_ids
            if let Some(parent) = state.records.get_mut(&self.task_id) {
                parent.child_task_ids = task_ids.clone();
            }
        } // Lock is released here

        // Notify that new tasks are ready
        self.notify.notify_one();

        Ok(task_ids)
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

        // Phase 5: Resolve dependencies for waiting tasks
        let waiting_tasks = state.dependency_graph.get_waiting_tasks(self.task_id);
        for waiting_task_id in waiting_tasks {
            // Remove this dependency from the waiting task
            if let Some(task) = state.records.get_mut(&waiting_task_id) {
                task.remove_dependency(self.task_id);

                // If the task has no more dependencies and is Queued, add to ready queue
                if !task.has_dependencies() && task.state == TaskState::Queued {
                    state.ready.push_back(waiting_task_id);
                }
            }

            // Remove from dependency graph
            state.dependency_graph.remove_dependency(waiting_task_id, self.task_id);
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
    use crate::{
        domain::{OutcomeKind, TaskId, TaskType, decision},
        queue,
    };

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
            TaskId::new(1001), // Note: enqueue() will allocate a new task_id
            TaskType::new("test_task"),
            serde_json::json!({"key": "value"}),
        );
        queue.enqueue(task).await.unwrap();
        let lease = queue.lease().await.unwrap();
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

    // Phase 4-1 tests: complete() with Decider flow

    #[tokio::test]
    async fn test_complete_with_retry_decision() {
        use crate::domain::{DefaultDecider, Outcome, OutcomeKind};
        use std::time::Duration;

        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task = TaskEnvelope::new(
            TaskId::new(2001), // Note: enqueue() will allocate a new task_id
            TaskType::new("test_task"),
            serde_json::json!({"test": "data"}),
        );
        queue.enqueue(task).await.unwrap();

        let lease = queue.lease().await.unwrap();

        // Create a failure outcome
        let outcome = Outcome::failure("test failure");

        // Create a retry decision
        let decision = Decision::Retry {
            delay: Duration::from_secs(5),
            reason: "retry test".to_string(),
        };

        // Call complete
        lease.complete(outcome, decision).await.unwrap();

        // Verify AttemptRecord was created
        let attempts = queue.get_all_attempts().await;
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].outcome.kind == OutcomeKind::Failure);

        // Get the actual task_id from the AttemptRecord (which has the correct allocated ID)
        let task_id = attempts[0].task_id;

        // Verify DecisionRecord was created
        let decisions = queue.get_decisions().await;
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "schedule_retry");
        assert_eq!(decisions[0].policy, "retry_policy");

        // Verify task was scheduled for retry
        let state = queue.state.lock().await;
        let record = state.records.get(&task_id).unwrap();
        assert_eq!(record.state, TaskState::RetryScheduled);
        assert_eq!(state.scheduled.len(), 1);
    }

    #[tokio::test]
    async fn test_complete_with_mark_dead_decision() {
        use crate::domain::{DefaultDecider, Outcome, OutcomeKind};

        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task = TaskEnvelope::new(
            TaskId::new(2002), // Note: enqueue() will allocate a new task_id
            TaskType::new("test_task"),
            serde_json::json!({"test": "data"}),
        );
        queue.enqueue(task).await.unwrap();

        let lease = queue.lease().await.unwrap();

        // Create a failure outcome
        let outcome = Outcome::failure("final failure");

        // Create a mark_dead decision
        let decision = Decision::MarkDead {
            reason: "max attempts reached".to_string(),
        };

        // Call complete
        lease.complete(outcome, decision).await.unwrap();

        // Verify AttemptRecord was created
        let attempts = queue.get_all_attempts().await;
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].outcome.kind == OutcomeKind::Failure);

        // Get the actual task_id from the AttemptRecord (which has the correct allocated ID)
        let task_id = attempts[0].task_id;

        // Verify DecisionRecord was created
        let decisions = queue.get_decisions().await;
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "mark_dead");
        assert_eq!(decisions[0].policy, "retry_policy");

        // Verify task was marked dead
        let state = queue.state.lock().await;
        let record = state.records.get(&task_id).unwrap();
        assert_eq!(record.state, TaskState::Dead);
    }

    #[tokio::test]
    async fn test_complete_creates_both_records() {
        use crate::domain::Outcome;
        use std::time::Duration;

        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task = TaskEnvelope::new(
            TaskId::new(2003),
            TaskType::new("test_task"),
            serde_json::json!({"key": "value"}),
        );
        queue.enqueue(task).await.unwrap();

        let lease = queue.lease().await.unwrap();

        let outcome = Outcome {
            kind: OutcomeKind::Failure,
            reason: Some("test error".to_string()),
            artifacts: vec![Artifact::Stderr("error details".to_string())],
            retry_hint: None,
            alternatives: vec![],
            child_tasks: None,
        };

        let decision = Decision::Retry {
            delay: Duration::from_secs(10),
            reason: "retry with backoff".to_string(),
        };

        lease.complete(outcome.clone(), decision).await.unwrap();

        // Verify both AttemptRecord and DecisionRecord were created
        let attempts = queue.get_all_attempts().await;
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].observation.len(), 1);
        match &attempts[0].observation[0] {
            Artifact::Stderr(msg) => assert_eq!(msg, "error details"),
            _ => panic!("Expected Artifact::Stderr"),
        }

        let decisions = queue.get_decisions().await;
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].context.is_some());
        let context = decisions[0].context.as_ref().unwrap();
        assert_eq!(context["delay_secs"], 10);
    }

    // Phase 5 tests: Dependency resolution

    #[tokio::test]
    async fn test_dependency_blocks_task_from_ready_queue() {
        let queue = InMemoryQueue::new(RetryPolicy::default_v1());
        let task_a_id = TaskId::new(100);
        let task_b_id = TaskId::new(101);

        {
            let mut state = queue.state.lock().await;

            // Create task A (no dependencies)
            let envelope_a = TaskEnvelope::new(
                task_a_id,
                TaskType::new("task_a"),
                serde_json::json!({"name": "A"}),
            );
            let record_a = TaskRecord::new(envelope_a, 5);
            state.records.insert(task_a_id, record_a);
            state.ready.push_back(task_a_id);

            // Create task B with dependency on A
            let envelope_b = TaskEnvelope::new(
                task_b_id,
                TaskType::new("task_b"),
                serde_json::json!({"name": "B"}),
            );
            let mut record_b = TaskRecord::new(envelope_b, 5);
            record_b.add_dependency(task_a_id);
            state.records.insert(task_b_id, record_b);

            // Register dependency in graph
            state.dependency_graph.add_dependency(task_b_id, task_a_id);

            // B should NOT be in ready queue (has dependencies)
            assert_eq!(state.ready.len(), 1);
            assert_eq!(state.ready.front(), Some(&task_a_id));
        }

        // Verify counts
        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 2); // Both A and B are queued
        assert_eq!(counts.running, 0);
    }

    #[tokio::test]
    async fn test_dependency_resolution_on_task_completion() {
        let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));
        let task_a_id = TaskId::new(200);
        let task_b_id = TaskId::new(201);

        {
            let mut state = queue.state.lock().await;

            // Create task A
            let envelope_a = TaskEnvelope::new(
                task_a_id,
                TaskType::new("task_a"),
                serde_json::json!({"name": "A"}),
            );
            let record_a = TaskRecord::new(envelope_a, 5);
            state.records.insert(task_a_id, record_a);
            state.ready.push_back(task_a_id);

            // Create task B with dependency on A
            let envelope_b = TaskEnvelope::new(
                task_b_id,
                TaskType::new("task_b"),
                serde_json::json!({"name": "B"}),
            );
            let mut record_b = TaskRecord::new(envelope_b, 5);
            record_b.add_dependency(task_a_id);
            state.records.insert(task_b_id, record_b);

            // Register dependency in graph
            state.dependency_graph.add_dependency(task_b_id, task_a_id);
        }

        queue.notify.notify_one();

        // Lease task A (should be the only ready task)
        let lease_a = queue.lease().await.unwrap();
        assert_eq!(lease_a.envelope().task_id(), task_a_id);

        // Complete task A (success)
        lease_a.ack().await.unwrap();

        // Now task B should be in ready queue
        {
            let state = queue.state.lock().await;
            assert_eq!(state.ready.len(), 1);
            assert_eq!(state.ready.front(), Some(&task_b_id));

            // Verify B no longer has dependencies
            let record_b = state.records.get(&task_b_id).unwrap();
            assert!(!record_b.has_dependencies());
        }

        // Should be able to lease task B now
        queue.notify.notify_one();
        let lease_b = queue.lease().await.unwrap();
        assert_eq!(lease_b.envelope().task_id(), task_b_id);
    }

    #[tokio::test]
    async fn test_multiple_dependencies() {
        let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));
        let task_a_id = TaskId::new(300);
        let task_b_id = TaskId::new(301);
        let task_c_id = TaskId::new(302);

        {
            let mut state = queue.state.lock().await;

            // Create task A
            let envelope_a = TaskEnvelope::new(
                task_a_id,
                TaskType::new("task_a"),
                serde_json::json!({"name": "A"}),
            );
            state.records.insert(task_a_id, TaskRecord::new(envelope_a, 5));
            state.ready.push_back(task_a_id);

            // Create task B
            let envelope_b = TaskEnvelope::new(
                task_b_id,
                TaskType::new("task_b"),
                serde_json::json!({"name": "B"}),
            );
            state.records.insert(task_b_id, TaskRecord::new(envelope_b, 5));
            state.ready.push_back(task_b_id);

            // Create task C with dependencies on both A and B
            let envelope_c = TaskEnvelope::new(
                task_c_id,
                TaskType::new("task_c"),
                serde_json::json!({"name": "C"}),
            );
            let mut record_c = TaskRecord::new(envelope_c, 5);
            record_c.add_dependency(task_a_id);
            record_c.add_dependency(task_b_id);
            state.records.insert(task_c_id, record_c);

            // Register dependencies
            state.dependency_graph.add_dependency(task_c_id, task_a_id);
            state.dependency_graph.add_dependency(task_c_id, task_b_id);

            // Only A and B should be ready
            assert_eq!(state.ready.len(), 2);
        }

        queue.notify.notify_one();

        // Complete task A
        let lease_a = queue.lease().await.unwrap();
        assert_eq!(lease_a.envelope().task_id(), task_a_id);
        lease_a.ack().await.unwrap();

        // C should still not be ready (still depends on B)
        {
            let state = queue.state.lock().await;
            let record_c = state.records.get(&task_c_id).unwrap();
            assert!(record_c.has_dependencies());
            assert_eq!(record_c.depends_on.len(), 1);
            assert_eq!(record_c.depends_on[0], task_b_id);
        }

        // Complete task B
        queue.notify.notify_one();
        let lease_b = queue.lease().await.unwrap();
        assert_eq!(lease_b.envelope().task_id(), task_b_id);
        lease_b.ack().await.unwrap();

        // Now C should be ready
        {
            let state = queue.state.lock().await;
            let record_c = state.records.get(&task_c_id).unwrap();
            assert!(!record_c.has_dependencies());
            assert_eq!(state.ready.len(), 1);
            assert_eq!(state.ready.front(), Some(&task_c_id));
        }
    }

    #[tokio::test]
    async fn test_add_child_tasks_creates_children_correctly() {
        use crate::domain::{JobSpec, TaskType};

        let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));

        // Create and enqueue a parent task via Job (so it has job_id)
        let job_spec = JobSpec::new(vec![TaskSpec::new(
            "parent task",
            TaskType::new("parent_task"),
            serde_json::json!({"data": "parent"}),
        )]);
        queue.submit_job(job_spec).await.unwrap();

        // Lease the parent task
        let lease = queue.lease().await.unwrap();
        assert_eq!(lease.envelope().task_id(), TaskId::new(1));

        // Create child task specs
        let child_specs = vec![
            TaskSpec::new(
                "child 1",
                TaskType::new("child_task"),
                serde_json::json!({"index": 1}),
            ),
            TaskSpec::new(
                "child 2",
                TaskType::new("child_task"),
                serde_json::json!({"index": 2}),
            ),
        ];

        // Add child tasks
        let child_ids = lease.add_child_tasks(child_specs).await.unwrap();

        // Verify 2 children were created
        assert_eq!(child_ids.len(), 2);

        // Verify child IDs are sequential
        assert_eq!(child_ids[0], TaskId::new(2));
        assert_eq!(child_ids[1], TaskId::new(3));

        // Verify parent's child_task_ids were updated
        let state = queue.state.lock().await;
        let parent_record = state.records.get(&TaskId::new(1)).unwrap();
        assert_eq!(parent_record.child_task_ids.len(), 2);
        assert_eq!(parent_record.child_task_ids[0], TaskId::new(2));
        assert_eq!(parent_record.child_task_ids[1], TaskId::new(3));

        // Verify children have correct parent_task_id
        let child1_record = state.records.get(&TaskId::new(2)).unwrap();
        assert_eq!(child1_record.parent_task_id, Some(TaskId::new(1)));
        assert_eq!(child1_record.envelope.task_type().as_str(), "child_task");
        assert_eq!(child1_record.envelope.payload()["index"], 1);

        let child2_record = state.records.get(&TaskId::new(3)).unwrap();
        assert_eq!(child2_record.parent_task_id, Some(TaskId::new(1)));
        assert_eq!(child2_record.envelope.task_type().as_str(), "child_task");
        assert_eq!(child2_record.envelope.payload()["index"], 2);

        // Verify children inherited job_id from parent
        assert_eq!(child1_record.job_id, parent_record.job_id);
        assert_eq!(child2_record.job_id, parent_record.job_id);

        // Verify children inherited max_attempts from parent
        assert_eq!(child1_record.max_attempts, parent_record.max_attempts);
        assert_eq!(child2_record.max_attempts, parent_record.max_attempts);

        // Verify children are in Ready state
        assert_eq!(child1_record.state, TaskState::Queued);
        assert_eq!(child2_record.state, TaskState::Queued);

        // Drop lock before calling counts_by_state (which also needs the lock)
        drop(state);

        // Verify queue counts
        let counts = queue.counts_by_state().await.unwrap();
        assert_eq!(counts.queued, 2); // 2 children in ready queue
        assert_eq!(counts.running, 1); // parent is running (leased)
    }
}
