//! TypedRegistry - Handler の登録と管理
//!
//! # 実装予定
//! - **PR-3**: TypedRegistry の実装

use super::task::Task;
use super::handler::Handler;

/// TypedRegistry は型付き Handler を登録
///
/// # 使用例（予定）
/// ```ignore
/// let mut registry = TypedRegistry::new();
/// registry.register::<MyTask>(MyTaskHandler);
/// ```
///
/// # 内部実装
/// - `register::<T: Task>(handler: impl Handler<T>)` で登録
/// - 内部的に DynHandler に変換
/// - HashMap<String, Arc<dyn DynHandler>> で管理
pub struct TypedRegistry {
    // TODO(PR-3): フィールド定義
    // handlers: HashMap<String, Arc<dyn DynHandler>>
}

impl TypedRegistry {
    // TODO(PR-3): メソッド実装
    // - new()
    // - register<T: Task>(&mut self, handler: impl Handler<T>)
    // - get(&self, task_type: &str) -> Option<&Arc<dyn DynHandler>>
}
