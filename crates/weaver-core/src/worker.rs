use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::domain::{Decider, Outcome, OutcomeKind};
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
    ///
    /// Phase 4-1: Added `decider` parameter for Handler → Outcome → Decider flow.
    pub fn spawn(
        n: usize,
        queue: Arc<dyn Queue>,
        runtime: Arc<Runtime>,
        decider: Arc<dyn Decider>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let mut joins = Vec::with_capacity(n);
        for worker_id in 0..n {
            let q = Arc::clone(&queue);
            let rt = Arc::clone(&runtime);
            let dec = Arc::clone(&decider);
            let mut rx = shutdown_rx.clone();

            let join = tokio::spawn(async move {
                worker_loop(worker_id, q, rt, dec, &mut rx).await;
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
    decider: Arc<dyn Decider>,
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

        // Phase 4-1: Handler → Outcome → Decider → Decision flow
        let envelope = lease.envelope().clone();

        let outcome_result = runtime.execute(&envelope).await;

        match outcome_result {
            Ok(outcome) => match outcome.kind {
                OutcomeKind::Success => {
                    lease.ack().await.unwrap_or_else(|e| {
                        eprintln!("[worker-{worker_id}] ack failed: {}", e);
                    });
                }
                OutcomeKind::Failure | OutcomeKind::Blocked => {
                    let task_record = lease.get_task_record().await.unwrap_or_else(|e| {
                        panic!("[worker-{worker_id}] get_task_record failed: {}", e);
                    });
                    let decision = decider.decide(&task_record, &outcome);
                    lease.complete(outcome, decision).await.unwrap_or_else(|e| {
                        eprintln!("[worker-{worker_id}] complete failed: {}", e);
                    });
                }
            },
            Err(handler_error) => {
                // Convert infrastructure error to business failure outcome
                let outcome = Outcome {
                    kind: OutcomeKind::Failure,
                    artifacts: Vec::new(),
                    reason: Some(handler_error.to_string()),
                    retry_hint: None,
                    alternatives: Vec::new(),
                };
                let decision = decider.decide(
                    &lease.get_task_record().await.unwrap_or_else(|e| {
                        panic!("[worker-{worker_id}] get_task_record failed: {}", e);
                    }),
                    &outcome,
                );
                eprintln!("[worker-{worker_id}] handler error: {}", handler_error);
                if let Err(e) = lease.complete(outcome, decision).await {
                    eprintln!("[worker-{worker_id}] complete failed: {e}");
                }
            }
        }
    }
}
