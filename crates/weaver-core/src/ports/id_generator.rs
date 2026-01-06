//! IdGenerator port - ID 生成の抽象化
//!
//! # 実装予定
//! - **PR-2**: UlidGenerator（ULID ベース）

/// IdGenerator は分散システムで使える ID を生成
///
/// # ULID の特性
/// - 時刻でソート可能
/// - 分散環境で生成可能（調整不要）
/// - 128-bit（UUID 互換）
pub trait IdGenerator {
    // TODO(PR-2): メソッド定義
    // - fn generate_job_id(&self) -> JobId
    // - fn generate_task_id(&self) -> TaskId
    // - fn generate_attempt_id(&self) -> AttemptId
}
