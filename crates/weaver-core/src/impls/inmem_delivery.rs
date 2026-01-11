//! InMemoryDeliveryQueue - 開発用の配送キュー
//!
//! # 学習ポイント
//! - Mutex + Condvar による blocking pop
//! - Async での blocking 処理の扱い（spawn_blocking）
//! - namespace による複数キューの管理

use crate::domain::ids::TaskId;
use crate::ports::{DeliveryQueue, QueueError};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// InMemoryDeliveryQueue は開発用の配送キュー
///
/// # 実装詳細
/// - HashMap<String, VecDeque<TaskId>> で namespace ごとにキューを管理
/// - Mutex で排他制御
/// - Condvar で push 時の通知
///
/// # 使用例
/// ```ignore
/// let queue = InMemoryDeliveryQueue::new();
/// queue.push("default", task_id).await?;
/// let task = queue.pop("default", Duration::from_secs(5)).await?;
/// ```
pub struct InMemoryDeliveryQueue {
    /// namespace ごとのキュー
    queues: Arc<Mutex<HashMap<String, VecDeque<TaskId>>>>,
    /// push 時の通知用
    condvar: Arc<Condvar>,
}

impl InMemoryDeliveryQueue {
    /// 新しい InMemoryDeliveryQueue を作成
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
            condvar: Arc::new(Condvar::new()),
        }
    }
}

impl Default for InMemoryDeliveryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl DeliveryQueue for InMemoryDeliveryQueue {
    /// task_id をキューに追加
    ///
    /// # 実装
    /// 1. Mutex をロック
    /// 2. namespace のキューを取得（なければ作成）
    /// 3. task_id を push_back
    /// 4. Condvar で待機中のスレッドに通知
    async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError> {
        let queues = self.queues.clone();
        let condvar = self.condvar.clone();
        let ns = ns.to_string();

        // spawn_blocking で同期処理を実行（async context で Mutex を使うため）
        tokio::task::spawn_blocking(move || {
            let mut queues = queues.lock().unwrap();
            let queue = queues.entry(ns).or_default();
            queue.push_back(task_id);

            // 待機中のスレッドに通知
            condvar.notify_one();
        })
        .await
        .map_err(|e| QueueError::OperationFailed(format!("Push failed: {}", e)))?;

        Ok(())
    }

    async fn pop(&self, ns: &str, timeout: Duration) -> Result<Option<TaskId>, QueueError> {
        let queues = self.queues.clone();
        let condvar = self.condvar.clone();
        let ns = ns.to_string();
        tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            let mut guard = queues.lock().unwrap();
            loop {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    return Ok(None);
                }
                if let Some(queue) = guard.get_mut(&ns)
                    && let Some(task_id) = queue.pop_front()
                {
                    return Ok(Some(task_id));
                }
                let remaining = timeout.saturating_sub(elapsed);
                let (new_guard, result) = condvar.wait_timeout(guard, remaining).unwrap();
                guard = new_guard;

                if result.timed_out() {
                    return Ok(None);
                }
            }
        })
        .await
        .map_err(|e| QueueError::OperationFailed(format!("Pop failed: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::Instant;
    use ulid::Ulid;

    #[tokio::test]
    async fn test_push_pop_roundtrip() {
        let queue = InMemoryDeliveryQueue::new();
        let task_id = TaskId::from_ulid(Ulid::new());
        queue.push("default", task_id).await.unwrap();
        let popped = queue.pop("default", Duration::from_secs(1)).await.unwrap();
        assert_eq!(popped, Some(task_id));
    }

    #[tokio::test]
    async fn test_pop_timeout() {
        let queue = InMemoryDeliveryQueue::new();
        let start = Instant::now();
        let popped = queue
            .pop("default", Duration::from_millis(500))
            .await
            .unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(500));
        assert_eq!(popped, None);
    }

    #[tokio::test]
    async fn test_multiple_namespaces() {
        let queue = InMemoryDeliveryQueue::new();
        let task_id1 = TaskId::from_ulid(Ulid::new());
        let task_id2 = TaskId::from_ulid(Ulid::new());
        queue.push("ns1", task_id1).await.unwrap();
        queue.push("ns2", task_id2).await.unwrap();

        let popped1 = queue.pop("ns1", Duration::from_secs(1)).await.unwrap();
        let popped2 = queue.pop("ns2", Duration::from_secs(1)).await.unwrap();

        assert_eq!(popped1, Some(task_id1));
        assert_eq!(popped2, Some(task_id2));
    }

    #[tokio::test]
    async fn test_push_wakes_pop() {
        let queue = std::sync::Arc::new(InMemoryDeliveryQueue::new());
        let task_id = TaskId::from_ulid(Ulid::new());

        let pop_future = tokio::spawn({
            let queue = queue.clone();
            async move { queue.pop("default", Duration::from_secs(5)).await.unwrap() }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        queue.push("default", task_id).await.unwrap();

        let popped = pop_future.await.unwrap();
        assert_eq!(popped, Some(task_id));
    }
}
