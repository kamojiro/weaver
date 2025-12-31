//! Job record and status management.

use std::time::Instant;

use crate::queue::TaskState;

use super::ids::{JobId, TaskId};
use super::spec::JobSpec;

/// Job state (aggregated from tasks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    /// At least one task is running or queued.
    Running,

    /// All tasks completed successfully.
    Completed,

    /// Some tasks failed permanently (Dead), no tasks are runnable.
    Failed,

    /// Job was cancelled by user.
    Cancelled,
}

/// Job record: tracks a collection of tasks.
///
/// Design: Following the same pattern as TaskRecord (ADR-0001, ADR-0002).
/// - Single source of truth for Job metadata
/// - State transitions via methods (not direct field access)
#[derive(Debug, Clone)]
pub struct JobRecord {
    pub job_id: JobId,
    pub spec: JobSpec,
    pub state: JobState,

    /// Tasks belonging to this job.
    pub task_ids: Vec<TaskId>,

    /// Timestamps for observability.
    pub created_at: Instant,
    pub updated_at: Instant,
}

impl JobRecord {
    pub fn new(job_id: JobId, spec: JobSpec) -> Self {
        let now = Instant::now();
        Self {
            job_id,
            spec,
            state: JobState::Running,
            task_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a task to this job.
    pub fn add_task(&mut self, task_id: TaskId) {
        self.task_ids.push(task_id);
        self.updated_at = Instant::now();
    }

    /// Mark job as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.state = JobState::Cancelled;
        self.updated_at = Instant::now();
    }

    /// Update job state based on task states.
    pub fn update_state_from_tasks(&mut self, task_states: &[(TaskId, crate::queue::TaskState)]) {
        // TODO(human): Implement job state aggregation logic here
        let state = {
            if task_states.is_empty() {
                JobState::Running
            } else if task_states
                .iter()
                .all(|&(_, state)| state == TaskState::Succeeded)
            {
                JobState::Completed
            } else if task_states.iter().any(|&(_, state)| {
                matches!(
                    state,
                    TaskState::Running | TaskState::Queued | TaskState::RetryScheduled
                )
            }) {
                JobState::Running
            } else if task_states.iter().all(|&(_, state)| state.is_terminal())
                && task_states
                    .iter()
                    .any(|(_, state)| *state == TaskState::Dead)
            {
                JobState::Failed
            } else {
                JobState::Running // unreachable fallback
            }
        };
        self.state = state;
        self.updated_at = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::TaskState;
    use rstest::rstest;

    #[test]
    fn new_job_starts_as_running() {
        let spec = JobSpec::new(vec![]);
        let job = JobRecord::new(JobId::new(1), spec);
        assert_eq!(job.state, JobState::Running);
    }

    #[test]
    fn can_add_tasks() {
        let spec = JobSpec::new(vec![]);
        let mut job = JobRecord::new(JobId::new(1), spec);

        job.add_task(TaskId::new(10));
        job.add_task(TaskId::new(20));

        assert_eq!(job.task_ids.len(), 2);
        assert!(job.task_ids.contains(&TaskId::new(10)));
    }

    #[test]
    fn update_job_state_all_succeeded() {
        let spec = JobSpec::new(vec![]);
        let mut job = JobRecord::new(JobId::new(1), spec);

        let task_states = vec![
            (TaskId::new(1), TaskState::Succeeded),
            (TaskId::new(2), TaskState::Succeeded),
        ];

        job.update_state_from_tasks(&task_states);
        assert_eq!(job.state, JobState::Completed);
    }

    #[rstest]
    #[case::running(TaskState::Running)]
    #[case::queued(TaskState::Queued)]
    #[case::retry_scheduled(TaskState::RetryScheduled)]
    fn update_job_state_includes_running(#[case] running_task_state: TaskState) {
        let spec = JobSpec::new(vec![]);
        let mut job = JobRecord::new(JobId::new(1), spec);

        let task_states = vec![
            (TaskId::new(1), TaskState::Succeeded),
            (TaskId::new(2), running_task_state),
        ];

        job.update_state_from_tasks(&task_states);
        assert_eq!(job.state, JobState::Running);
    }

    #[test]
    fn update_job_state_includes_dead() {
        let spec = JobSpec::new(vec![]);
        let mut job = JobRecord::new(JobId::new(1), spec);
        let task_states = vec![
            (TaskId::new(1), TaskState::Succeeded),
            (TaskId::new(2), TaskState::Dead),
        ];

        job.update_state_from_tasks(&task_states);
        assert_eq!(job.state, JobState::Failed);
    }

    #[test]
    fn update_job_state_with_empty_tasks() {
        let spec = JobSpec::new(vec![]);
        let mut job = JobRecord::new(JobId::new(1), spec);
        let task_states = vec![];

        job.update_state_from_tasks(&task_states);
        assert_eq!(job.state, JobState::Running);
    }
}
