//! DispatchStrategy port - task_type から handler_id へのマッピング
//!
//! # 実装予定
//! - **PR-4**: DirectDispatch（task_type == handler_id）
//! - **v3**: RuleDispatch, AgentDispatch

use crate::domain::errors::WeaverError;

/// DispatchStrategy は task_type を handler_id に解決
///
/// # v2 デフォルト
/// - DirectDispatch: 1:1 マッピング（task_type == handler_id）
///
/// # 将来の拡張
/// - RuleDispatch: ルールベースのマッピング
/// - AgentDispatch: LLM による動的マッピング
pub trait DispatchStrategy {
    // TODO(human): メソッド定義
    // select_handler(&self, task_type: &str) -> Result<String, WeaverError> を定義してください
    //
    // ヒント:
    // - &self を使うことで object-safe になります
    // - task_type は &str（借用）、戻り値は String（所有）
    // - Result を返すことで、将来のエラーケースに対応します
    //
    // 詳細は dev/learning/learning_2026_01_07.md の「PR-4: DispatchStrategy」セクションを参照
}
