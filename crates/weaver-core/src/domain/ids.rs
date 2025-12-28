//! Domain identifiers (strongly-typed IDs).
//!
//! # Why not `u64` everywhere?
//! Using newtype IDs prevents mixing different identifiers by mistake.
//! (e.g. passing a `TaskId` where a `JobId` is expected).
//!
//! We derive `Serialize/Deserialize` so that future persistence / external APIs
//! can use the same types without refactoring.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier of a Job (submit/status/cancel/result unit).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JobId(u64);

impl JobId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job-{}", self.0)
    }
}

/// Identifier of a Task (trackable unit within a Job).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task-{}", self.0)
    }
}

/// Identifier of an Attempt (one execution try of a Task).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AttemptId(u64);

impl AttemptId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for AttemptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "attempt-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_types() {
        let job = JobId::new(1);
        let task = TaskId::new(1);
        let attempt = AttemptId::new(1);

        assert_eq!(job.get(), 1);
        assert_eq!(task.get(), 1);
        assert_eq!(attempt.get(), 1);

        // Display is stable and human-friendly
        assert_eq!(job.to_string(), "job-1");
        assert_eq!(task.to_string(), "task-1");
        assert_eq!(attempt.to_string(), "attempt-1");

        // The whole point: you can't accidentally mix these types.
        // (This is a compile-time property, so we just keep it as a comment.)
        // let _: JobId = task; // <- does not compile
    }
}
