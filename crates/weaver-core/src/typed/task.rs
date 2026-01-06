//! Task trait - 型付き Task の定義
//!
//! # 実装予定
//! - **PR-3**: Task trait の実装

/// Task は task_type と型を対応付ける
///
/// # 使用例（予定）
/// ```ignore
/// #[derive(Serialize, Deserialize)]
/// struct MyTask {
///     message: String,
/// }
///
/// impl Task for MyTask {
///     const TYPE: &'static str = "my_namespace.my_task.v1";
/// }
/// ```
pub trait Task {
    /// task_type の定義
    ///
    /// # 命名規約
    /// - `{namespace}.{domain}.{action}.v{major}`
    /// - 例: `acme.billing.charge.v1`
    const TYPE: &'static str;

    // TODO(PR-3): 追加のメソッド（必要に応じて）
}
