//! Clock port - 時刻の抽象化
//!
//! Clock trait は現在時刻を提供するためのインターフェースです。
//! テスト容易性のために、trait として抽象化しています。
//!
//! # 実装
//! - **SystemClock**: 本番用（`Utc::now()` を呼ぶ）
//! - **FixedClock**: テスト用（固定時刻を返す）

use chrono::{DateTime, Utc};

/// Clock は現在時刻を提供
///
/// # テスト容易性
/// - trait により時刻を差し替え可能
/// - テストでは FixedClock を使用して決定的なテストを書ける
///
/// # Thread Safety
/// - `Send + Sync` を要求（複数スレッドから使える）
pub trait Clock: Send + Sync {
    /// 現在時刻を返す
    fn now(&self) -> DateTime<Utc>;
}

/// SystemClock は本番用の Clock 実装
///
/// `Utc::now()` を呼んで現在時刻を返します。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// FixedClock はテスト用の Clock 実装
///
/// 固定された時刻を常に返します。これにより、時刻に依存するテストを
/// 決定的に実行できます。
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    time: DateTime<Utc>,
}

impl FixedClock {
    /// 固定時刻を指定して FixedClock を作成
    pub fn new(time: DateTime<Utc>) -> Self {
        Self { time }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn system_clock_returns_current_time() {
        let clock = SystemClock;
        let now = clock.now();

        // 現在時刻が返されることを確認（厳密な値は確認できないので、範囲チェック）
        let before = Utc::now();
        let actual = clock.now();
        let after = Utc::now();

        assert!(actual >= before);
        assert!(actual <= after);
    }

    #[test]
    fn fixed_clock_returns_fixed_time() {
        let fixed_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FixedClock::new(fixed_time);

        // 何度呼んでも同じ時刻が返される
        assert_eq!(clock.now(), fixed_time);
        assert_eq!(clock.now(), fixed_time);
        assert_eq!(clock.now(), fixed_time);
    }

    #[test]
    fn fixed_clock_is_deterministic() {
        let time1 = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2024, 6, 15, 18, 30, 45).unwrap();

        let clock1 = FixedClock::new(time1);
        let clock2 = FixedClock::new(time2);

        assert_eq!(clock1.now(), time1);
        assert_eq!(clock2.now(), time2);
        assert_ne!(clock1.now(), clock2.now());
    }
}
