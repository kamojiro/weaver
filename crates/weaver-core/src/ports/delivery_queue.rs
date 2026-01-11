//! DeliveryQueue port - 配送キュー（Redis または InMemory）
//!
//! DeliveryQueue は task_id のみを流します（状態や payload は含まない）。
//!
//! # v2 の設計
//! - Redis（または InMemory）は task_id のみを保持
//! - 状態・payload・envelope は PostgreSQL に保存
//! - namespace をサポート（マルチテナント対応）

use crate::domain::ids::TaskId;
use std::time::Duration;

/// DeliveryQueue は task_id を配送するためのキュー
///
/// # 設計原則
/// - task_id のみを保持（状態・payload・envelope は PG に保存）
/// - namespace をサポート（マルチテナント対応）
/// - blocking pop（timeout 付き）
///
/// # 実装
/// - **InMemoryDeliveryQueue**: 開発用（VecDeque + Mutex/Condvar）
/// - **RedisDeliveryQueue**: 本番用（RPUSH/BLPOP）
#[async_trait::async_trait]
pub trait DeliveryQueue: Send + Sync {
    /// task_id をキューに追加
    ///
    /// # Arguments
    /// - `ns`: namespace（例: "default"）
    /// - `task_id`: 配送する task_id
    async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError>;

    /// task_id をキューから取り出す（blocking + timeout）
    ///
    /// # Arguments
    /// - `ns`: namespace（例: "default"）
    /// - `timeout`: タイムアウト時間
    ///
    /// # Returns
    /// - `Ok(Some(task_id))`: task_id を取得
    /// - `Ok(None)`: timeout まで待っても要素なし
    /// - `Err(QueueError)`: エラー
    async fn pop(&self, ns: &str, timeout: Duration) -> Result<Option<TaskId>, QueueError>;
}

/// QueueError は DeliveryQueue の操作エラー
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue operation failed: {0}")]
    OperationFailed(String),
}
