//! AppBuilder - アプリケーションの構築とワイヤリング
//!
//! # 実装予定
//! - **PR-5**: 起動時検証（expect_tasks）

/// AppBuilder はアプリケーションを構築
///
/// # 使用例（予定）
/// ```ignore
/// let app = AppBuilder::new()
///     .task_store(pg_store)
///     .delivery_queue(redis_queue)
///     .artifact_store(minio_store)
///     .register::<MyTask>(MyTaskHandler)
///     .expect_tasks(&["my_namespace.my_task.v1"])
///     .build()?;
/// ```
pub struct AppBuilder {
    // TODO(PR-5): フィールド定義
}

impl AppBuilder {
    // TODO(PR-5): メソッド実装
    // - new()
    // - task_store()
    // - delivery_queue()
    // - artifact_store()
    // - register()
    // - expect_tasks()
    // - build()
}
