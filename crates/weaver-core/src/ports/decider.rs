//! Decider port - Outcome から Decision を生成
//!
//! Decider は純粋関数として設計されます（副作用なし）。
//!
//! # 実装予定
//! - v2 では基本的な Decider を実装
//! - 将来的には chain of deciders をサポート可能

/// Decider は Outcome と状態から Decision を生成
///
/// # 設計原則
/// - 純粋関数（current_state + observation → next_action）
/// - 副作用なし（実行は Runner に任せる）
pub trait Decider {
    // TODO(v2後半): メソッド定義
    // - fn decide(&self, outcome: Outcome, context: DecisionContext) -> Decision
}
