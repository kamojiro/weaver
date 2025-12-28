use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{TaskEnvelope, TaskType};
use crate::error::WeaverError;

/// A handler for a specific task type.
///
/// v1: take the whole `TaskEnvelope` so the handler can decode payload as it likes.
/// (JSON decode, bytes, etc.)
#[async_trait]
pub trait TaskHandler: Send + Sync {
    async fn handle(&self, envelope: &TaskEnvelope) -> Result<(), WeaverError>;
}

/// Registry of handlers (task_type -> handler).
///
/// Design:
/// - Built during initialization (mutable).
/// - Used during runtime (immutable).
/// This avoids locks and keeps it simple for v1.
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<TaskType, Arc<dyn TaskHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a task type.
    ///
    /// If you want "last wins", change this to overwrite instead of error.
    pub fn register(
        &mut self,
        task_type: TaskType,
        handler: Arc<dyn TaskHandler>,
    ) -> Result<(), WeaverError> {
        if self.handlers.contains_key(&task_type) {
          return Err(WeaverError::DuplicateHandler(task_type));
        }
        self.handlers.insert(task_type, handler);
        Ok(())
    }

    pub fn get(&self, task_type: &TaskType) -> Option<&Arc<dyn TaskHandler>> {
        self.handlers.get(task_type)
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

/// Runtime executes a `TaskEnvelope` by dispatching to a registered handler.
pub struct Runtime {
    registry: Arc<HandlerRegistry>,
}

impl Runtime {
    pub fn new(registry: Arc<HandlerRegistry>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &HandlerRegistry {
        &self.registry
    }

    /// Execute one envelope.
    pub async fn execute(&self, envelope: &TaskEnvelope) -> Result<(), WeaverError> {
        let task_type = envelope.task_type();
        let handler = self
            .registry
            .get(&task_type)
            .ok_or_else(|| WeaverError::HandlerNotFound(task_type.clone()))?;

        handler.handle(envelope).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskEnvelope, TaskId};

    struct OkHandler;

    #[async_trait]
    impl TaskHandler for OkHandler {
        async fn handle(&self, _envelope: &TaskEnvelope) -> Result<(), WeaverError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn runtime_executes_registered_handler() {
        let mut reg = HandlerRegistry::new();
        reg.register(TaskType::new("ok"), Arc::new(OkHandler))
            .unwrap();

        let rt = Runtime::new(Arc::new(reg));

        let env = TaskEnvelope::new(TaskId::new(1), TaskType::new("ok"), serde_json::json!({}));
        rt.execute(&env).await.unwrap();
    }

    #[tokio::test]
    async fn runtime_errors_when_handler_missing() {
        let rt = Runtime::new(Arc::new(HandlerRegistry::new()));

        let env = TaskEnvelope::new(TaskId::new(1), TaskType::new("missing"), serde_json::json!({}));
        let err = rt.execute(&env).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("handler"));
    }
}
