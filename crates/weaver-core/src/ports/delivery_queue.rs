//! DeliveryQueue port - 配送キュー（Redis または InMemory）
//!
//! DeliveryQueue は task_id のみを流します（状態や payload は含まない）。
//!
//! # 実装予定
//! - **PR-6**: InMemoryDeliveryQueue（開発用）
//! - **PR-9**: RedisDeliveryQueue（本番用）

/// DeliveryQueue は task_id を配送するためのキュー
///
/// # 設計原則
/// - task_id のみを保持（状態・payload・envelope は PG に保存）
/// - namespace をサポート（マルチテナント対応）
/// - blocking pop（timeout 付き）
pub trait DeliveryQueue {
    // TODO(PR-6, PR-9): メソッド定義
    // - async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError>
    // - async fn pop(&self, ns: &str, timeout: Duration) -> Result<Option<TaskId>, QueueError>
}
