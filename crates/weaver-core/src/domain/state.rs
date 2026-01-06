//! State - タスクとジョブの状態
//!
//! # 実装予定
//! - Phase 2: 既存 job.rs の JobState などを移動
//! - v2: TaskState の定義

/// TaskState はタスクの状態を表現
///
/// # 状態遷移（v2）
/// - pending: 依存待ち
/// - ready: 実行可能
/// - running: 実行中（lease 発行済み）
/// - succeeded: 成功
/// - failed: 失敗
/// - blocked: ブロック（repair 待ちなど）
/// - cancelled: キャンセル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Blocked,
    Cancelled,
}

/// JobState はジョブの状態を表現
///
/// # 状態遷移（v2）
/// - running: 実行中
/// - completed: 完了
/// - failed: 失敗
/// - cancelled: キャンセル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// WaitingReason は pending/blocked の詳細理由
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitingReason {
    /// 依存関係待ち
    DependenciesPending,
    /// Repair 待ち
    RepairPending,
    /// 予算切れ
    BudgetExhausted,
    /// その他
    Other(String),
}
