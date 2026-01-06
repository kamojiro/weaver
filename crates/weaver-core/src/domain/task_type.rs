//! TaskType - task_type 命名規約のサポート
//!
//! # 実装予定
//! - v2 後半: バリデータ、命名規約チェック

/// TaskType は task_type の命名規約をサポート
///
/// # 命名規約
/// - `{namespace}.{domain}.{action}.v{major}`
/// - 例: `acme.billing.charge.v1`
///
/// # 将来の拡張
/// - バリデータ: 命名規約に従っているかチェック
/// - パーサー: namespace, domain, action, version を抽出
pub struct TaskType {
    // TODO(v2後半): フィールド定義
    // value: String
}

impl TaskType {
    // TODO(v2後半): メソッド実装
    // - new(value: String) -> Result<Self, ValidationError>
    // - validate(value: &str) -> Result<(), ValidationError>
    // - parse(value: &str) -> Result<ParsedTaskType, ParseError>
}
