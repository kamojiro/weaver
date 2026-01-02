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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DefaultDecider, Outcome, TaskEnvelope, TaskId, TaskType};
    use crate::queue::{InMemoryQueue, RetryPolicy};
    use crate::runtime::{HandlerRegistry, TaskHandler};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::{sleep, Duration};

    /// Test handler that fails N times before succeeding
    struct FailingHandler {
        remaining_failures: AtomicU32,
    }

    impl FailingHandler {
        fn new(n: u32) -> Self {
            Self {
                remaining_failures: AtomicU32::new(n),
            }
        }
    }

    #[async_trait]
    impl TaskHandler for FailingHandler {
        async fn handle(&self, _envelope: &TaskEnvelope) -> Result<Outcome, crate::error::WeaverError> {
            let left = self.remaining_failures.load(Ordering::Relaxed);
            if left > 0 {
                self.remaining_failures.fetch_sub(1, Ordering::Relaxed);
                return Ok(Outcome::failure(format!("intentional failure (left={left})")));
            }
            Ok(Outcome::success())
        }
    }

    #[tokio::test]
    async fn test_worker_retry_flow_integration() {
        // Setup: Queue, Runtime with FailingHandler, DefaultDecider, WorkerGroup
        let queue = Arc::new(InMemoryQueue::new(RetryPolicy {
            base_delay: Duration::from_millis(50), // Short delay for test
            multiplier: 1.0,                       // No exponential backoff
        }));

        let mut registry = HandlerRegistry::new();
        registry
            .register(
                TaskType::new("failing_task"),
                Arc::new(FailingHandler::new(2)), // Fails 2 times, succeeds on 3rd
            )
            .unwrap();
        let runtime = Arc::new(Runtime::new(Arc::new(registry)));

        let decider = Arc::new(DefaultDecider::new(RetryPolicy {
            base_delay: Duration::from_millis(50),
            multiplier: 1.0,
        }));

        // Start 1 worker
        let workers = WorkerGroup::spawn(1, queue.clone(), runtime.clone(), decider);

        // Enqueue a task
        let envelope = TaskEnvelope::new(
            TaskId::new(1),
            TaskType::new("failing_task"),
            serde_json::json!({}),
        );
        queue.enqueue(envelope).await.unwrap();

        // Wait for task to complete (should retry twice and succeed on 3rd attempt)
        // Each retry has ~50ms delay, so total time should be ~150ms
        for _ in 0..30 {
            // Max 3 seconds wait
            let counts = queue.counts_by_state().await.unwrap();
            if counts.succeeded > 0 {
                // Success!
                assert_eq!(counts.succeeded, 1);
                assert_eq!(counts.dead, 0);
                assert_eq!(counts.running, 0);
                assert_eq!(counts.retry_scheduled, 0);

                // Verify attempt records were created
                let attempts = queue.get_all_attempts().await;
                assert_eq!(
                    attempts.len(),
                    3,
                    "Should have 3 attempts (2 failures + 1 success)"
                );

                // Shutdown workers
                workers.shutdown_and_join().await;
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }

        panic!("Task did not complete successfully within timeout");
    }

    #[tokio::test]
    async fn test_worker_max_attempts_exceeded() {
        // Setup: Queue, Runtime with always-failing handler, DefaultDecider
        // Note: max_attempts is hardcoded to 5 in TaskRecord (see memory.rs:207)
        let queue = Arc::new(InMemoryQueue::new(RetryPolicy {
            base_delay: Duration::from_millis(10),
            multiplier: 1.0,
        }));

        let mut registry = HandlerRegistry::new();
        registry
            .register(
                TaskType::new("always_failing"),
                Arc::new(FailingHandler::new(100)), // Fails 100 times (more than max_attempts=5)
            )
            .unwrap();
        let runtime = Arc::new(Runtime::new(Arc::new(registry)));

        let decider = Arc::new(DefaultDecider::new(RetryPolicy {
            base_delay: Duration::from_millis(10),
            multiplier: 1.0,
        }));

        // Start 1 worker
        let workers = WorkerGroup::spawn(1, queue.clone(), runtime.clone(), decider);

        // Enqueue a task
        let envelope = TaskEnvelope::new(
            TaskId::new(1),
            TaskType::new("always_failing"),
            serde_json::json!({}),
        );
        queue.enqueue(envelope).await.unwrap();

        // Wait for task to be marked dead
        for _ in 0..30 {
            let counts = queue.counts_by_state().await.unwrap();
            if counts.dead > 0 {
                // Task should be dead after max_attempts
                assert_eq!(counts.dead, 1);
                assert_eq!(counts.succeeded, 0);

                // Verify attempt records
                let attempts = queue.get_all_attempts().await;
                assert_eq!(
                    attempts.len(),
                    5,
                    "Should have exactly max_attempts (5) attempts"
                );

                // Shutdown workers
                workers.shutdown_and_join().await;
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }

        panic!("Task was not marked dead within timeout");
    }

    #[tokio::test]
    async fn test_worker_immediate_success() {
        // Setup: Handler that succeeds immediately
        let queue = Arc::new(InMemoryQueue::new(RetryPolicy::default_v1()));

        let mut registry = HandlerRegistry::new();
        registry
            .register(
                TaskType::new("success_task"),
                Arc::new(FailingHandler::new(0)), // Succeeds immediately
            )
            .unwrap();
        let runtime = Arc::new(Runtime::new(Arc::new(registry)));

        let decider = Arc::new(DefaultDecider::default_v1());

        // Start 1 worker
        let workers = WorkerGroup::spawn(1, queue.clone(), runtime.clone(), decider);

        // Enqueue a task
        let envelope = TaskEnvelope::new(
            TaskId::new(1),
            TaskType::new("success_task"),
            serde_json::json!({}),
        );
        queue.enqueue(envelope).await.unwrap();

        // Wait for task to complete
        for _ in 0..10 {
            let counts = queue.counts_by_state().await.unwrap();
            if counts.succeeded > 0 {
                assert_eq!(counts.succeeded, 1);
                assert_eq!(counts.dead, 0);

                // Should have only 1 attempt (success on first try)
                let attempts = queue.get_all_attempts().await;
                assert_eq!(attempts.len(), 1, "Should have only 1 attempt");

                // Shutdown workers
                workers.shutdown_and_join().await;
                return;
            }
            sleep(Duration::from_millis(50)).await;
        }

        panic!("Task did not complete successfully within timeout");
    }
}
