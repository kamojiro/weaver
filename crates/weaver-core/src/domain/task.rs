use serde::{Deserialize, Serialize};
use std::fmt;

use super::TaskId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskType(String);

impl TaskType {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// TaskType + Payload (+ TaskId) の“運搬用”データ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEnvelope {
    task_id: TaskId,
    task_type: TaskType,
    payload: serde_json::Value,
}

impl TaskEnvelope {
    pub fn new(task_id: TaskId, task_type: TaskType, payload: serde_json::Value) -> Self {
        Self {
            task_id,
            task_type,
            payload,
        }
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
}
