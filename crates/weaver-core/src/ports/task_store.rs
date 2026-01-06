//! TaskStore port - PostgreSQL が実装する正本（source of truth）
//!
//! TaskStore は以下を管理します：
//! - 状態（TaskState, JobState）
//! - 履歴（attempts, decisions）
//! - 依存関係（task_dependencies）
//! - 配送指示（outbox_events）
//!
//! # 実装予定
//! - **PR-7**: `weaver-pg` クレートで PostgreSQL 実装
//! - テスト用に InMemory 実装も検討

/// TaskStore は状態・履歴・依存・outbox の正本（source of truth）
///
/// # 設計原則
/// - 状態遷移（claim/complete/reap）と outbox 生成は同一トランザクション内
/// - Lease の権威はここにある（Redis の pop は候補通知に過ぎない）
/// - すべての状態は PostgreSQL から再構築可能
pub trait TaskStore {
    // TODO(PR-7): メソッド定義
    // - create_job / create_task / add_dependency
    // - claim (lease 発行)
    // - complete (状態更新・履歴記録・依存解放・outbox生成)
    // - evaluate_readiness (ready 再評価)
    // - reap_expired_leases (期限切れ回収)
    // - update_payload (repair 用)
    // - pull_outbox / ack_outbox / fail_outbox
}
