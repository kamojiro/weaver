use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{Duration, sleep};

use weaver_core::domain::{TaskEnvelope, TaskId, TaskType};
use weaver_core::error::WeaverError;
use weaver_core::queue::{InMemoryQueue, Queue, RetryPolicy};
use weaver_core::runtime::{HandlerRegistry, Runtime, TaskHandler};
use weaver_core::worker::WorkerGroup;

#[derive(Debug, Deserialize)]
struct HelloPayload {
    name: String,
}

/// HelloHandler: 意図的に2回失敗してから成功するハンドラー
struct HelloHandler {
    remaining_failures: AtomicU32,
}

impl HelloHandler {
    fn new(n: u32) -> Self {
        Self {
            remaining_failures: AtomicU32::new(n),
        }
    }
}

#[async_trait]
impl TaskHandler for HelloHandler {
    async fn handle(&self, envelope: &TaskEnvelope) -> Result<(), WeaverError> {
        // Payload を JSON として decode
        let p: HelloPayload = serde_json::from_value(envelope.payload().clone())
            .map_err(|e| WeaverError::Other(format!("json decode: {e}")))?;

        let left = self.remaining_failures.load(Ordering::Relaxed);
        if left > 0 {
            self.remaining_failures.fetch_sub(1, Ordering::Relaxed);
            return Err(WeaverError::Other(format!(
                "intentional failure (left={left})"
            )));
        }

        println!("✓ Hello, {}!", p.name);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    println!("=== Weaver CLI Example ===\n");

    // (A) Queue と HandlerRegistry を用意
    let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));

    let mut reg = HandlerRegistry::new();
    reg.register(TaskType::new("hello"), Arc::new(HelloHandler::new(2)))
        .expect("register handler");
    let runtime = Arc::new(Runtime::new(Arc::new(reg)));

    // (B) Worker を起動（1本）
    let workers = WorkerGroup::spawn(1, queue.clone(), runtime.clone());

    // (C) タスク投入
    let task_id = TaskId::new(1); // 手動でIDを割り当て（本来はQueueが管理）
    let env = TaskEnvelope::new(
        task_id,
        TaskType::new("hello"),
        serde_json::json!({ "name": "Weaver" }),
    );

    queue.enqueue(env).await.expect("enqueue");
    println!("📤 Enqueued task: {}\n", task_id);

    // (D) 完了をポーリングで待つ
    // TODO: 本来は get_status(TaskId) API を実装すべきだが、
    // v1では counts_by_state() で全体の状態を見る
    loop {
        let counts = queue.counts_by_state().await.expect("counts");

        println!(
            "📊 State counts: queued={}, running={}, succeeded={}, retry_scheduled={}, dead={}",
            counts.queued, counts.running, counts.succeeded, counts.retry_scheduled, counts.dead
        );

        // 終了条件: succeeded か dead のいずれかになったら
        if counts.succeeded > 0 || counts.dead > 0 {
            println!("\n✅ Task completed!");
            if counts.succeeded > 0 {
                println!("   Result: SUCCESS");
            } else {
                println!("   Result: DEAD (max retries exceeded)");
            }
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }

    // (E) Worker を graceful shutdown
    workers.shutdown_and_join().await;
    println!("\n👋 Shutdown complete");
}
