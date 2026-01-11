//! TypedRegistry - Handler の登録と管理
//!
//! # 学習ポイント
//! - HashMap での型消去された trait object の管理
//! - Generic methods での登録と型安全性
//! - Arc による共有所有権

use crate::typed::handler::TypedHandler;

use super::handler::{DynHandler, Handler};
use super::task::Task;
use std::collections::HashMap;
use std::sync::Arc;

/// TypedRegistry は型付き Handler を登録・管理
///
/// # 使用例
/// ```ignore
/// let mut registry = TypedRegistry::new();
/// registry.register::<MyTask>(MyTaskHandler)?;
///
/// // task_type で DynHandler を取得
/// let handler = registry.get("my_app.my_task.v1")?;
/// ```
///
/// # 内部実装
/// - `register::<T: Task>(handler: impl Handler<T>)` で登録
/// - 内部的に TypedHandler でラップして DynHandler に変換
/// - HashMap<String, Arc<dyn DynHandler>> で管理
pub struct TypedRegistry {
    handlers: HashMap<String, Arc<dyn DynHandler>>,
}

/// RegistryError は TypedRegistry の操作エラー
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Handler for task type '{0}' is already registered")]
    AlreadyRegistered(String),
}

impl TypedRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<T: Task, H: Handler<T> + 'static>(
        &mut self,
        handler: H,
    ) -> Result<(), RegistryError> {
        let task_type = T::TYPE.to_string();
        if self.handlers.contains_key(&task_type) {
            return Err(RegistryError::AlreadyRegistered(task_type));
        }
        let typed_handler = TypedHandler::new(handler);
        self.handlers.insert(task_type, Arc::new(typed_handler));
        Ok(())
    }

    pub fn get(&self, task_type: &str) -> Option<Arc<dyn DynHandler>> {
        self.handlers.get(task_type).cloned()
    }

    pub fn registered_types(&self) -> Vec<String>{
        self.handlers.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::task::{TestTask, AnotherTestTask};
    use crate::typed::handler::{TestTaskHandler, AnotherTestTaskHandler};


    #[test]   
    fn test_register_and_get() {
        let mut registry = TypedRegistry::new();
        let handler = TestTaskHandler{};
        registry.register::<TestTask, _>(handler).unwrap();

        let retrieved = registry.get(TestTask::TYPE);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_double_registration() {
        let mut registry = TypedRegistry::new();
        let handler1 = TestTaskHandler{};
        let handler2 = TestTaskHandler{};
        registry.register::<TestTask, _>(handler1).unwrap();
        let result = registry.register::<TestTask, _>(handler2);
        assert!(matches!(result, Err(RegistryError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_registered_types() {
        let mut registry = TypedRegistry::new();
        let handler = TestTaskHandler{};
        registry.register::<TestTask, _>(handler).unwrap();
        let types = registry.registered_types();
        assert_eq!(types, vec![TestTask::TYPE.to_string()]);
    }

    #[test]
    fn test_different_task_types() {
        let mut registry = TypedRegistry::new();
        let test_handler = TestTaskHandler{};
        let another_handler = AnotherTestTaskHandler{};

        registry.register::<TestTask, _>(test_handler).unwrap();
        registry.register::<AnotherTestTask, _>(another_handler).unwrap();

        let retrieved_test = registry.get(TestTask::TYPE);
        let retrieved_another = registry.get(AnotherTestTask::TYPE);

        assert!(retrieved_test.is_some());
        assert!(retrieved_another.is_some());
    }
}