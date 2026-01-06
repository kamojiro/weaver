//! GCLoop - Artifact のガベージコレクション
//!
//! # 実装予定
//! - **PR-12**: expires_at < now の artifact を削除

/// GCLoop は期限切れの artifact を削除
///
/// # フロー
/// 1. 定期的に expires_at < now の artifact を検索
/// 2. PG の deleted_at を更新
/// 3. Blob から削除
pub struct GCLoop {
    // TODO(PR-12): フィールド定義
}

impl GCLoop {
    // TODO(PR-12): メソッド実装
    // - new()
    // - run()
}
