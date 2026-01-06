//! Events - ドメインイベント
//!
//! # 実装予定
//! - v2 最小: 基本的なイベント定義
//! - 将来: EventSink への送信

/// DomainEvent はドメインで発生したイベント
///
/// # イベント種類（予定）
/// - TaskCreated
/// - TaskClaimed
/// - TaskCompleted
/// - TaskFailed
/// - JobCompleted
#[derive(Debug, Clone)]
pub enum DomainEvent {
    // TODO(v2): イベント定義
    // TaskCreated { ... },
    // TaskClaimed { ... },
    // TaskCompleted { ... },
}

impl DomainEvent {
    // TODO(v2): メソッド実装
}
