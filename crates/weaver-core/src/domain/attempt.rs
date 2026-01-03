//! Attempt and Decision models for execution history.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::ids::{AttemptId, TaskId};
use super::outcome::{Artifact, Outcome};

/// A single execution attempt of a task.
///
/// Records:
/// - What was done (action)
/// - What was observed (observation)
/// - What happened (outcome)
///
/// This is the foundation of "explain why" capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt_id: AttemptId,
    pub task_id: TaskId,

    /// What action was taken (flexible JSON for v1).
    /// Examples: command executed, HTTP request, function call, etc.
    pub action: serde_json::Value,

    /// What was observed during execution.
    /// Examples: stdout, stderr, response body, metrics, etc.
    pub observation: Vec<Artifact>,

    /// The result of this attempt.
    pub outcome: Outcome,

    /// When this attempt started (not serialized in v1).
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub started_at: Instant,

    /// When this attempt completed (or failed/blocked) (not serialized in v1).
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub completed_at: Instant,
}

impl AttemptRecord {
    /// Create a new attempt record.
    /// (In production, you'd track actual start/completion times separately)
    pub fn new(
        attempt_id: AttemptId,
        task_id: TaskId,
        action: serde_json::Value,
        observation: Vec<Artifact>,
        outcome: Outcome,
    ) -> Self {
        Self {
            attempt_id,
            task_id,
            action,
            observation,
            outcome,
            started_at: Instant::now(),
            completed_at: Instant::now(),
        }
    }
}

/// A decision made during execution.
///
/// Records:
/// - What observation/context led to this decision
/// - What policy/strategy was used
/// - What action was taken (retry, decompose, add dependency, stop, etc.)
///
/// This enables "why did the system do X" explanations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub task_id: TaskId,

    /// What was observed that triggered this decision.
    /// Could reference an AttemptId, or be a general observation.
    pub trigger: serde_json::Value,

    /// What policy/strategy was applied.
    /// Examples: "retry_with_backoff", "decompose_large_task", "stuck_detection"
    pub policy: String,

    /// What action was decided.
    /// Examples: "schedule_retry", "create_subtasks", "mark_blocked", "cancel"
    pub decision: String,

    /// Additional context (flexible for v1).
    pub context: Option<serde_json::Value>,

    /// When this decision was made (not serialized in v1).
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub decided_at: Instant,
}

impl DecisionRecord {
    /// Create a new decision record.
    pub fn new(
        task_id: TaskId,
        trigger: serde_json::Value,
        policy: impl Into<String>,
        decision: impl Into<String>,
        context: Option<serde_json::Value>,
    ) -> Self {
        Self {
            task_id,
            trigger,
            policy: policy.into(),
            decision: decision.into(),
            context,
            decided_at: Instant::now(),
        }
    }
}
