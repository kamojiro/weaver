//! DispatchStrategy port - task_type から handler_id へのマッピング
//!
//! # 実装予定
//! - **PR-4**: DirectDispatch（task_type == handler_id）
//! - **v3**: RuleDispatch, AgentDispatch

/// DispatchStrategy は task_type を handler_id に解決
///
/// # v2 デフォルト
/// - DirectDispatch: 1:1 マッピング（task_type == handler_id）
///
/// # 将来の拡張
/// - RuleDispatch: ルールベースのマッピング
/// - AgentDispatch: LLM による動的マッピング
pub trait DispatchStrategy {
    // TODO(PR-4): メソッド定義
    // - fn select_handler(&self, task_type: &str, meta: &TaskMeta) -> Result<String, DispatchError>
}
