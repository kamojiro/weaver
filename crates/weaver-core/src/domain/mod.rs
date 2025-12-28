//! Domain model (IDs, specs, outcomes, records, ...).
pub mod ids;
pub mod outcome;
pub mod spec;
pub mod task;

pub use ids::{AttemptId, JobId, TaskId};
pub use outcome::{Artifact, Outcome, OutcomeKind};
pub use spec::{Budget, JobSpec, TaskSpec};
pub use task::{TaskEnvelope, TaskType};
