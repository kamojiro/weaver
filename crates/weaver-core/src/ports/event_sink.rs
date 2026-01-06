//! EventSink port - イベント記録の抽象化
//!
//! # 実装予定
//! - v2 最小: NoopEventSink（何もしない）
//! - 将来: Kafka, CloudWatch Logs などへの送信

/// EventSink はドメインイベントを記録
///
/// # v2 最小実装
/// - NoopEventSink: 何もしない（オプショナル機能）
///
/// # 将来の拡張
/// - Kafka へのイベント送信
/// - CloudWatch Logs への記録
pub trait EventSink {
    // TODO(v2後半): メソッド定義
    // - async fn emit(&self, event: DomainEvent) -> Result<(), EventSinkError>
}
