//! DirectDispatch - task_type と handler_id を 1:1 でマッピング
//!
//! # v2 デフォルト実装
//! DirectDispatch は最もシンプルな DispatchStrategy 実装です。

use crate::domain::errors::WeaverError;
use crate::ports::DispatchStrategy;

pub struct DirectDispatch;

impl DirectDispatch {
    pub fn new() -> Self {
        Self
    }
}

impl DispatchStrategy for DirectDispatch {
    fn select_handler(&self, task_type: &str) -> Result<String, WeaverError> {
        Ok(task_type.to_string())
    }
}

impl Default for DirectDispatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_dispatch() {
        let dispatcher = DirectDispatch::new();
        let task_type = "example_task";
        let handler_id = dispatcher.select_handler(task_type).unwrap();
        assert_eq!(handler_id, task_type);
    }
}
