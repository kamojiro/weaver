//! Outcome model: common result format for attempts.
//!
//! This module is architecture-agnostic: it does not assume queues, workers,
//! or persistence. It only defines the "shape" of results that the system can
//! record and explain later.

use serde::{Deserialize, Serialize};

use super::spec::TaskSpec;

/// A unified classification of an attempt result.
///
/// We intentionally serialize as SCREAMING_SNAKE_CASE to match the requirement:
/// SUCCESS / FAILURE / BLOCKED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeKind {
    Success,
    Failure,
    Blocked,
}

/// A reference to something produced or observed during execution.
///
/// Keep this flexible: artifacts are used in explanation/reporting and can be
/// extended without changing the core execution model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Artifact {
    /// Standard output captured from a command, etc.
    Stdout(String),

    /// Standard error captured from a command, etc.
    Stderr(String),

    /// Path to a file produced/used.
    FilePath(String),

    /// URL reference (e.g., a web resource).
    Url(String),

    /// Arbitrary JSON payload (structured observation/output).
    Json(serde_json::Value),
}

/// A common result format for an attempt.
///
/// - `SUCCESS`: forward progress happened (can be final or intermediate).
/// - `FAILURE`: recoverable failure (retry / alternatives / decomposition possible).
/// - `BLOCKED`: cannot proceed without additional info/prerequisites/interaction.
///
/// v1 keeps "hints" as JSON to avoid over-constraining the action schema too early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub kind: OutcomeKind,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Optional hint for retry (e.g., recommended delay, missing data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_hint: Option<serde_json::Value>,

    /// Optional alternative actions/approaches (domain-specific).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_tasks: Option<Vec<TaskSpec>>,
}

impl Outcome {
    pub fn success() -> Self {
        Self {
            kind: OutcomeKind::Success,
            artifacts: Vec::new(),
            reason: None,
            retry_hint: None,
            alternatives: Vec::new(),
            child_tasks: None,
        }
    }

    pub fn failure(reason: impl Into<String>) -> Self {
        Self {
            kind: OutcomeKind::Failure,
            artifacts: Vec::new(),
            reason: Some(reason.into()),
            retry_hint: None,
            alternatives: Vec::new(),
            child_tasks: None,
        }
    }

    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            kind: OutcomeKind::Blocked,
            artifacts: Vec::new(),
            reason: Some(reason.into()),
            retry_hint: None,
            alternatives: Vec::new(),
            child_tasks: None,
        }
    }

    pub fn with_artifact(mut self, artifact: Artifact) -> Self {
        self.artifacts.push(artifact);
        self
    }

    pub fn with_retry_hint(mut self, hint: serde_json::Value) -> Self {
        self.retry_hint = Some(hint);
        self
    }

    pub fn with_alternative(mut self, alternative: serde_json::Value) -> Self {
        self.alternatives.push(alternative);
        self
    }

    pub fn with_decompose_hint(mut self, child_tasks: Vec<TaskSpec>) -> Self {
        self.child_tasks = Some(child_tasks);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_kind_serializes_as_required_names() {
        let s = serde_json::to_string(&OutcomeKind::Success).unwrap();
        assert_eq!(s, "\"SUCCESS\"");

        let s = serde_json::to_string(&OutcomeKind::Failure).unwrap();
        assert_eq!(s, "\"FAILURE\"");

        let s = serde_json::to_string(&OutcomeKind::Blocked).unwrap();
        assert_eq!(s, "\"BLOCKED\"");
    }

    #[test]
    fn outcome_roundtrip_json() {
        let o = Outcome::failure("oops")
            .with_artifact(Artifact::Stderr("E".to_string()))
            .with_retry_hint(serde_json::json!({"delay_ms": 1000}))
            .with_alternative(serde_json::json!({"action": "try_other"}));

        let s = serde_json::to_string(&o).unwrap();
        let back: Outcome = serde_json::from_str(&s).unwrap();
        assert_eq!(back.kind, OutcomeKind::Failure);
        assert_eq!(back.reason.as_deref(), Some("oops"));
        assert_eq!(back.artifacts.len(), 1);
        assert!(back.retry_hint.is_some());
        assert_eq!(back.alternatives.len(), 1);
    }

    #[test]
    fn artifact_is_tagged_enum() {
        let a = Artifact::Stdout("hello".to_string());
        let s = serde_json::to_string(&a).unwrap();
        // Example shape: {"kind":"Stdout","value":"hello"}
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["kind"], "Stdout");
        assert_eq!(v["value"], "hello");
    }
}
