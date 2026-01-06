//! PublisherLoop - Outbox イベントの配送
//!
//! # 実装予定
//! - **PR-8**: outbox→push→ack

/// PublisherLoop は PG の outbox を読んで DeliveryQueue に配送
///
/// # フロー
/// 1. TaskStore::pull_outbox() で pending イベントを取得
/// 2. DeliveryQueue::push() で配送
/// 3. TaskStore::ack_outbox() で sent にマーク
/// 4. エラー時は TaskStore::fail_outbox() でリトライ
pub struct PublisherLoop {
    // TODO(PR-8): フィールド定義
}

impl PublisherLoop {
    // TODO(PR-8): メソッド実装
    // - new()
    // - run()
}
