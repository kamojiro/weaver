//! Handler trait - Task を実行する Handler の定義
//!
//! # 実装予定
//! - **PR-3**: Handler trait の実装

use super::task::Task;

/// Handler は Task を実行して Outcome を返す
///
/// # 使用例（予定）
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
pub trait Handler<T: Task> {
    // TODO(PR-3): メソッド定義
    // - async fn handle(&self, task: T) -> Result<Outcome, WeaverError>
}

/// DynHandler は object-safe な Handler の抽象化
///
/// TypedHandler<T> を DynHandler に変換することで、
/// HashMap<String, Arc<dyn DynHandler>> に格納可能にする
pub trait DynHandler {
    // TODO(PR-3): メソッド定義
    // - async fn handle_dyn(&self, artifact: ArtifactRef) -> Result<Outcome, WeaverError>
}
