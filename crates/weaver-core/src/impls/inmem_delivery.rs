//! InMemoryDeliveryQueue - 開発用の配送キュー
//!
//! # 実装予定
//! - **PR-6**: InMemoryDeliveryQueue の実装
//!
//! # 用途
//! - 開発時の動作確認（PG/Redis なしで動作）
//! - テスト用

use crate::ports::DeliveryQueue;

/// InMemoryDeliveryQueue は開発用の配送キュー
///
/// # 実装詳細
/// - VecDeque + Mutex/RwLock での実装
/// - namespace 対応
/// - blocking pop（timeout 付き）
pub struct InMemoryDeliveryQueue {
    // TODO(PR-6): フィールド定義
    // queues: Arc<RwLock<HashMap<String, VecDeque<TaskId>>>>
}

impl InMemoryDeliveryQueue {
    // TODO(PR-6): メソッド実装
    // - new()
}

impl DeliveryQueue for InMemoryDeliveryQueue {
    // TODO(PR-6): trait 実装
    // - async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError>
    // - async fn pop(&self, ns: &str, timeout: Duration) -> Result<Option<TaskId>, QueueError>
}
