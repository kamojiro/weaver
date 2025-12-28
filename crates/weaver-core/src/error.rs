use thiserror::Error;

use crate::domain::TaskType;

#[derive(Debug, Error)]
pub enum WeaverError {
    #[error("handler not found for task_type={0}")]
    HandlerNotFound(TaskType),

    #[error("duplicate handler for task_type={0}")]
    DuplicateHandler(TaskType),

    #[error("{0}")]
    Other(String),
}

