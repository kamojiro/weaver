//! Domain model (IDs, specs, outcomes, records, ...).
//!
//! v2 モジュール構成への移行中:
//! - 新規: task_type, envelope, budget, state, errors, events
//! - 既存（v1互換）: attempt, decision, ids, job, outcome, spec, task

// v2 の新しいモジュール
pub mod task_type;
pub mod envelope;
pub mod budget;
pub mod state;
pub mod errors;
pub mod events;

// v1 の既存モジュール（段階的に移行予定）
pub mod attempt;
pub mod decision;
pub mod ids;
pub mod job;
pub mod outcome;
pub mod spec;
pub mod task;

// v2 の型を再エクスポート
pub use self::task_type::TaskType as TaskTypeV2;
pub use self::envelope::TaskEnvelope as TaskEnvelopeV2;
pub use self::budget::Budget as BudgetV2;
pub use self::state::{TaskState, JobState as JobStateV2, WaitingReason};
pub use self::errors::{ErrorKind, WeaverError};
pub use self::events::DomainEvent;

// v1 の型を再エクスポート（互換性維持）
pub use attempt::{AttemptRecord, DecisionRecord};
pub use decision::{Decision, Decider, DefaultDecider};
pub use ids::{AttemptId, JobId, TaskId};
pub use job::{JobRecord, JobResult, JobState, JobStateView, JobStatus};
pub use outcome::{Artifact, Outcome, OutcomeKind};
pub use spec::{Budget, JobSpec, TaskSpec};
pub use task::{TaskEnvelope, TaskType};
