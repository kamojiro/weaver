//! Input specs for Weaver (Job / Task / Budget).
//!
//! These are intentionally flexible for v1. We represent many open-ended fields
//! as `serde_json::Value` so we can evolve without breaking changes.

use serde::{Deserialize, Serialize};

/// A Job is the unit of submission / cancellation / status / result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    pub tasks: Vec<TaskSpec>,

    /// Budget that applies to the whole job (optional / partial in v1).
    #[serde(default)]
    pub budget: Budget,
}

/// A trackable unit inside a job.
/// Tasks may be added during execution (decomposition, alternatives, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    /// Human-readable title.
    pub title: Option<String>,

    /// Human-readable intent / explanation.
    pub intent: Option<String>,

    /// Goal/definition-of-done (domain-specific; keep flexible in v1).
    pub goal: Option<serde_json::Value>,

    /// Constraints specific to this task (timeouts, priority, etc.).
    pub constraints: Option<serde_json::Value>,

    /// Initial hint for how to execute this task.
    /// This can be a `task_type + payload` style, or a higher-level action schema.
    pub seed_action_hint: Option<serde_json::Value>,

    /// Optional initial dependencies (TaskIds may not be known at creation time;
    /// for v1 we keep this flexible as JSON).
    pub dependencies_hint: Option<serde_json::Value>,
}

impl TaskSpec {
    /// Convenience constructor for simple "one task" use cases.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            intent: None,
            goal: None,
            constraints: None,
            seed_action_hint: None,
            dependencies_hint: None,
        }
    }
}

/// Execution budgets / stop conditions.
/// v1: Keep it minimal and easy to extend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum attempts per task (including retries).
    pub max_attempts_per_task: u32,

    /// Optional maximum attempts across the whole job.
    pub max_total_attempts: Option<u32>,

    /// Optional job-level deadline (milliseconds since start).
    pub deadline_ms: Option<u64>,

    /// Optional "stuck" detection: stop if no progress for N ticks/events.
    pub max_no_progress_steps: Option<u32>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_attempts_per_task: 5,
            max_total_attempts: None,
            deadline_ms: None,
            max_no_progress_steps: Some(50),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_default_is_reasonable() {
        let b = Budget::default();
        assert_eq!(b.max_attempts_per_task, 5);
        assert_eq!(b.max_no_progress_steps, Some(50));
    }

    #[test]
    fn job_spec_roundtrip_json() {
        let job = JobSpec {
            tasks: vec![TaskSpec::new("hello")],
            budget: Budget::default(),
        };

        let s = serde_json::to_string(&job).expect("serialize");
        let de: JobSpec = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(de.tasks.len(), 1);
        assert_eq!(de.tasks[0].title.as_deref(), Some("hello"));
    }

    #[test]
    fn job_spec_without_budget_then_get_default_budget(){
      let json = r#"
      {
        "tasks": [
          { "title": "hello" }
        ]
      }"#;
      let job: JobSpec = serde_json::from_str(json).expect("deserialize");
      assert_eq!(job.budget.max_attempts_per_task, 5);
    }
}
