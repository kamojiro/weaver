use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::error::WeaverError;
use crate::queue::Queue;
use crate::runtime::Runtime;

/// Worker group handle.
/// - `shutdown_tx` を drop するとワーカー全体が止まる
/// - `join()` で全ワーカーの終了を待てる
pub struct WorkerGroup {
    shutdown_tx: watch::Sender<bool>,
    joins: Vec<JoinHandle<()>>,
}

impl WorkerGroup {
    /// Spawn `n` workers.
    pub fn spawn(n: usize, queue: Arc<dyn Queue>, runtime: Arc<Runtime>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut joins = Vec::with_capacity(n);
        for worker_id in 0..n {
            let q = Arc::clone(&queue);
            let rt = Arc::clone(&runtime);
            let mut rx = shutdown_rx.clone();

            let join = tokio::spawn(async move {
                worker_loop(worker_id, q, rt, &mut rx).await;
            });
            joins.push(join);
        }

        Self { shutdown_tx, joins }
    }

    /// Request shutdown for all workers.
    /// This does not forcibly cancel in-flight handler execution; it just stops
    /// taking new leases. (v1 方針に合う)
    pub fn request_shutdown(&self) {
        // ignore send error: receivers may already be dropped
        let _ = self.shutdown_tx.send(true);
    }

    /// Shutdown and wait for all workers.
    pub async fn shutdown_and_join(self) {
        self.request_shutdown();
        for j in self.joins {
            let _ = j.await;
        }
    }
}

async fn worker_loop(
    worker_id: usize,
    queue: Arc<dyn Queue>,
    runtime: Arc<Runtime>,
    shutdown_rx: &mut watch::Receiver<bool>,
) {
    loop {
        // shutdown が来ていたら抜ける
        if *shutdown_rx.borrow() {
            break;
        }

        // lease は「待つ」可能性があるので select で shutdown と競合させる
        let lease = tokio::select! {
            _ = shutdown_rx.changed() => {
                // 変更が入ったら次のループで判定
                continue;
            }
            lease = queue.lease() => lease,
        };

        let Some(lease) = lease else {
            // Queue 側が「いま何もない」を None で返す設計なら少し待つ
            // (すでに内部で待つ設計なら、この分岐自体が不要)
            tokio::task::yield_now().await;
            continue;
        };

        // ここから先は handler 実行（await がある）
        // 重要: Queue 内部ロックは lease() の中で完結している前提（ロック跨ぎ await しない）
        let envelope = lease.envelope().clone(); // handler 実行に必要な分だけ owned にする

        let result: Result<(), WeaverError> = runtime.execute(&envelope).await;

        match result {
            Ok(_outcome_or_unit) => {
                // 成功を queue に反映
                if let Err(e) = lease.ack().await {
                    eprintln!("[worker-{worker_id}] ack failed: {e}");
                }
            }
            Err(err) => {
                // 失敗を queue に反映（queue が retry/dead を判断するのが基本方針）
                if let Err(e) = lease.fail(err.to_string()).await {
                    eprintln!("[worker-{worker_id}] fail report failed: {e}");
                }
            }
        }
    }
}
