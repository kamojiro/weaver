//! Budget - 予算管理（試行回数、期限、コスト制限）
//!
//! # 実装予定
//! - Phase 2: 既存 spec.rs の Budget を移動

/// Budget は実行の制約を定義
///
/// # フィールド（予定）
/// - max_attempts: Option<u32>
/// - deadline: Option<DateTime<Utc>>
/// - max_cost: Option<Decimal>（将来）
pub struct Budget {
    // TODO(Phase 2): 既存 spec.rs から移動
}

impl Budget {
    // TODO(Phase 2): メソッド実装
}
