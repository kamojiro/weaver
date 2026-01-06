//! RepairHintGenerator port - decode 失敗時のヒント生成
//!
//! # 実装予定
//! - **PR-13**: NoopRepairHintGenerator（v2最小）
//! - **v3**: LLM による自動修復ヒント

/// RepairHintGenerator は decode 失敗時にヒントを生成
///
/// # v2 最小実装
/// - NoopRepairHintGenerator: 空のヒントを返す
///
/// # 将来の拡張
/// - LLM ベースの自動修復ヒント生成
pub trait RepairHintGenerator {
    // TODO(PR-13): メソッド定義
    // - async fn hint(&self, input: RepairHintInput) -> Result<RepairHint, RepairError>
}
