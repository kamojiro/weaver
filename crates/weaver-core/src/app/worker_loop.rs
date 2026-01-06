//! WorkerLoop - タスク実行ループ
//!
//! # 実装予定
//! - **PR-10**: pop→claim→handle→decide→complete

/// WorkerLoop はタスクを実行
///
/// # フロー
/// 1. DeliveryQueue::pop() で task_id 取得
/// 2. TaskStore::claim() で lease 発行 + TaskEnvelope 取得
/// 3. PayloadCodec で deserialize
/// 4. Handler 実行 → Outcome
/// 5. Decider 実行 → Decision
/// 6. TaskStore::complete() で状態更新・履歴記録・依存解放・outbox生成
pub struct WorkerLoop {
    // TODO(PR-10): フィールド定義
}

impl WorkerLoop {
    // TODO(PR-10): メソッド実装
    // - new()
    // - run()
}
