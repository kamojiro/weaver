//! TaskEnvelope - タスクの実行コンテキスト
//!
//! # 実装予定
//! - Phase 2: 既存 task.rs の TaskEnvelope を移動

/// TaskEnvelope はタスクの実行に必要な全情報
///
/// # フィールド（予定）
/// - task_type: String
/// - artifact_ref: ArtifactRef（payload への参照）
/// - schema_version: String（minor/patch version）
/// - meta: TaskMeta（namespace, job_id, parent_id など）
pub struct TaskEnvelope {
    // TODO(Phase 2): 既存 task.rs から移動
}

impl TaskEnvelope {
    // TODO(Phase 2): メソッド実装
}
