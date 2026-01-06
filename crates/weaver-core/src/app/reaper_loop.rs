//! ReaperLoop - Lease 期限切れの回収
//!
//! # 実装予定
//! - **PR-11**: reap_expired_leases→再評価→outbox

/// ReaperLoop は lease が期限切れになったタスクを回収して再配送
///
/// # フロー
/// 1. TaskStore::reap_expired_leases() で期限切れを取得
/// 2. running → pending/ready へ遷移
/// 3. ready になったら outbox に dispatch_task を追加
pub struct ReaperLoop {
    // TODO(PR-11): フィールド定義
}

impl ReaperLoop {
    // TODO(PR-11): メソッド実装
    // - new()
    // - run()
}
