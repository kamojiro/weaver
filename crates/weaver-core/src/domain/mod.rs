//! Domain model (IDs, specs, outcomes, records, ...).
pub mod attempt;
pub mod ids;
pub mod job;
pub mod outcome;
pub mod spec;
pub mod task;

pub use attempt::{AttemptRecord, DecisionRecord};
pub use ids::{AttemptId, JobId, TaskId};
pub use job::{JobRecord, JobState};
pub use outcome::{Artifact, Outcome, OutcomeKind};
pub use spec::{Budget, JobSpec, TaskSpec};
pub use task::{TaskEnvelope, TaskType};
