//! Clock port - 時刻の抽象化
//!
//! # 実装予定
//! - **PR-2**: SystemClock（本番用）、FixedClock（テスト用）

/// Clock は現在時刻を提供
///
/// # テスト容易性
/// - trait により時刻を差し替え可能
/// - テストでは FixedClock を使用
pub trait Clock {
    // TODO(PR-2): メソッド定義
    // - fn now(&self) -> DateTime<Utc>
}
