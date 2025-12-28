use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use std::sync::atomic::{AtomicU32, Ordering};
use weaver_core::domain::{ContentType, RetryPolicy, TaskEnvelope, TaskState};
use weaver_core::queue::{InMemoryQueue, Queue};
use weaver_core::runtime::{HandlerRegistry, TaskHandler, execute_one};

#[derive(Debug, Deserialize)]
struct HelloPayload {
    name: String,
}

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
    fn task_type(&self) -> &'static str {
        "hello"
    }

    async fn execute(&self, payload: &[u8]) -> Result<(), String> {
        let p: HelloPayload =
            serde_json::from_slice(payload).map_err(|e| format!("json decode: {e}"))?;

        let left = self.remaining_failures.load(Ordering::Relaxed);
        if left > 0 {
            self.remaining_failures.fetch_sub(1, Ordering::Relaxed);
            return Err(format!("intentional failure (left={left})"));
        }

        println!("Hello, {}!", p.name);
        Ok(())
    }
}

/// worker：Queue と Runtime をつなぐ接着剤
async fn worker_loop(queue: Arc<InMemoryQueue>, registry: Arc<HandlerRegistry>) {
    loop {
        // 1) 実行できるタスクを1件取る（state: Queued -> Running）
        let lease = queue.lease().await;
        println!("leased: id={:?} attempt={}", lease.id, lease.attempt);

        // 2) task_type から handler を引いて実行
        let result = execute_one(&registry, &lease.envelope).await;

        // 3) 結果で state を更新（成功: Succeeded / 失敗: retry or dead は Queue 側）
        match result {
            Ok(()) => queue.ack(lease.id).await,
            Err(e) => queue.fail(lease.id, e.to_string()).await,
        }
    }
}

#[tokio::main]
async fn main() {
    // (A) Queue と HandlerRegistry を用意
    let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));

    let mut reg = HandlerRegistry::new();
    reg.register(Arc::new(HelloHandler::new(2)));
    let reg = Arc::new(reg);

    // (B) worker を起動（今回は 1 本）
    let worker = tokio::spawn(worker_loop(queue.clone(), reg.clone()));

    // (C) タスク投入（TaskType + Payload(JSON bytes)）
    let env = TaskEnvelope {
        task_type: "hello",
        payload: serde_json::to_vec(&serde_json::json!({ "name": "weaver" })).unwrap(),
        content_type: ContentType::Json,
    };
    let id = queue.enqueue(env).await;
    println!("enqueued task: {:?}", id);

    // (D) 完了をポーリングで待つ（Succeeded / Dead のどちらか）
    loop {
        let st = queue.get_status(id).await.expect("task exists");
        if matches!(st.state, TaskState::Succeeded | TaskState::Dead) {
            println!(
                "final status: state={:?} attempts={} last_error={:?}",
                st.state, st.attempts, st.last_error
            );
            println!("counts: {:?}", queue.counts_by_state().await);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    // (E) サンプルなので worker を止める（本番は graceful shutdown を設計する）
    worker.abort();
}
