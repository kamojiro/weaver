//! Handler trait - Task を実行する Handler の定義
//!
//! # 学習ポイント
//! - ジェネリック trait (Handler<T>)
//! - Object-safe trait (DynHandler)
//! - Type erasure パターン (TypedHandler<T, H> → DynHandler)

use super::task::{Task, TestTask, AnotherTestTask};
use crate::domain::errors::WeaverError;
use crate::domain::outcome::Outcome;
use async_trait::async_trait;
use std::marker::PhantomData;

/// Handler は Task を実行して Outcome を返す
///
/// # 使用例
/// ```ignore
/// struct MyTaskHandler;
///
/// #[async_trait]
/// impl Handler<MyTask> for MyTaskHandler {
///     async fn handle(&self, task: MyTask) -> Result<Outcome, WeaverError> {
///         println!("Processing: {}", task.message);
///         Ok(Outcome::success())
///     }
/// }
/// ```
///
/// # ジェネリクスによる型安全性
/// - `Handler<TestTask>` は `TestTask` しか受け取れない
/// - コンパイル時に Task と Handler の対応が保証される
#[async_trait]
pub trait Handler<T: Task>: Send + Sync {
    async fn handle(&self, task: T) -> Result<Outcome, WeaverError>;
}

/// DynHandler は object-safe な Handler の抽象化
///
/// TypedHandler<T> を DynHandler に変換することで、
/// HashMap<String, Arc<dyn DynHandler>> に格納可能にします。
///
/// # Object Safety
/// - メソッドはジェネリックではない（具体的な型のみ）
/// - `dyn DynHandler` として trait object にできる
#[async_trait]
pub trait DynHandler: Send + Sync {
    async fn handle_dyn(&self, payload: serde_json::Value) -> Result<Outcome, WeaverError>;
    fn task_type(&self) -> &str;
}


pub struct TypedHandler<T: Task, H: Handler<T>> {
    handler: H,
    _marker: PhantomData<T>,
}

impl<T: Task, H: Handler<T>> TypedHandler<T, H> {
    pub fn new(handler: H) -> Self {
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<T: Task, H: Handler<T>> DynHandler for TypedHandler<T, H> {
    async fn handle_dyn(&self, payload: serde_json::Value) -> Result<Outcome, WeaverError> {
        let task: T = serde_json::from_value(payload)
            .map_err(|e| WeaverError::new(format!("json decode: {e}")))?;
        self.handler.handle(task).await
    }

    fn task_type(&self) -> &str {
        T::TYPE
    }
}

pub struct TestTaskHandler;

#[async_trait]
impl Handler<TestTask> for TestTaskHandler {
    async fn handle(&self, _task: TestTask) -> Result<Outcome, WeaverError> {
        Ok(Outcome::success())
    }
}

pub struct AnotherTestTaskHandler;

#[async_trait]
impl Handler<AnotherTestTask> for AnotherTestTaskHandler {
    async fn handle(&self, _task: AnotherTestTask) -> Result<Outcome, WeaverError> {
        Ok(Outcome::success())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::OutcomeKind;

    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_typed_handler() {
        let handler = TestTaskHandler;
        let typed_handler = TypedHandler::<TestTask, _>::new(handler);

        let payload = json!({ "value": 100 });
        let outcome = typed_handler.handle_dyn(payload).await.unwrap();
        assert!(outcome.kind == OutcomeKind::Success);
    }
}