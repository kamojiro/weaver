//! Decision model: next action determination for task execution.
//!
//! This module defines the Decision type (what to do next) and the Decider trait
//! (how to determine the next action based on task state and outcome).

use std::time::Duration;

use super::Outcome;
use crate::queue::{RetryPolicy, TaskRecord};

/// The next action to take for a task.
///
/// Phase 4-1: Minimal decision types (retry/stop only)
/// Phase 4-2: Will add Decompose, AddDependency, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Retry the task after a delay.
    Retry {
        delay: Duration,
        reason: String,
    },

    /// Mark the task as dead (give up).
    MarkDead {
        reason: String,
    },
}

/// Trait for deciding the next action based on task state and outcome.
///
/// Deciders are pure functions: given the current state and observation,
/// they return the next action without side effects.
///
/// Phase 4-1: weaver-core provides DefaultDecider (retry/budget logic)
/// Future: Users can implement custom Deciders (AI agents, domain logic, etc.)
pub trait Decider: Send + Sync {
    /// Decide the next action for a task based on its current state and outcome.
    ///
    /// # Arguments
    /// * `task` - The task record (contains attempts, max_attempts, etc.)
    /// * `outcome` - The outcome of the most recent attempt
    ///
    /// # Returns
    /// The next action to take (Retry or MarkDead)
    fn decide(&self, task: &TaskRecord, outcome: &Outcome) -> Decision;
}

/// Default decider provided by weaver-core.
///
/// Implements attempt-based retry logic with exponential backoff:
/// - Retry if attempts < max_attempts
/// - Mark dead if attempts >= max_attempts
/// - Use RetryPolicy for delay calculation
///
/// This is a pure function implementation - no side effects, no state mutation.
/// The actual execution of the Decision (updating TaskRecord, scheduling retry)
/// is handled by TaskLease/Worker.
#[derive(Debug, Clone)]
pub struct DefaultDecider {
    retry_policy: RetryPolicy,
}

impl DefaultDecider {
    /// Create a new DefaultDecider with the given retry policy.
    pub fn new(retry_policy: RetryPolicy) -> Self {
        Self { retry_policy }
    }

    /// Create a DefaultDecider with v1 default policy (2s base, 2.0 multiplier).
    pub fn default_v1() -> Self {
        Self::new(RetryPolicy::default_v1())
    }
}

impl Decider for DefaultDecider {
    fn decide(&self, task: &TaskRecord, _outcome: &Outcome) -> Decision {
        if task.attempts >= task.max_attempts {
            Decision::MarkDead {
                reason: format!(
                    "Max attempts reached: {}/{}",
                    task.attempts, task.max_attempts
                ),
            }
        } else {
            let delay = self.retry_policy.next_delay(task.attempts);
            Decision::Retry {
                delay,
                reason: format!(
                    "Retry attempt {}/{} after {:?}",
                    task.attempts + 1,
                    task.max_attempts,
                    delay
                ),
            }
        }
    }
}
