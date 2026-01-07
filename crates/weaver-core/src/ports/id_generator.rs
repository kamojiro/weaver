//! IdGenerator port - ID 生成の抽象化
//!
//! IdGenerator は分散システムで使える ID を生成するためのインターフェースです。
//! テスト容易性のために、trait として抽象化しています。
//!
//! # 実装
//! - **UlidGenerator**: ULID ベース（本番用）

use crate::domain::ids::{AttemptId, JobId, TaskId};
use crate::ports::Clock;
use ulid::Ulid;

/// IdGenerator は分散システムで使える ID を生成
///
/// # ULID の特性
/// - 時刻でソート可能
/// - 分散環境で生成可能（調整不要）
/// - 128-bit（UUID 互換）
///
/// # Thread Safety
/// - `Send + Sync` を要求（複数スレッドから使える）
pub trait IdGenerator: Send + Sync {
    /// Job ID を生成
    fn generate_job_id(&self) -> JobId;

    /// Task ID を生成
    fn generate_task_id(&self) -> TaskId;

    /// Attempt ID を生成
    fn generate_attempt_id(&self) -> AttemptId;
}

/// UlidGenerator は ULID ベースの ID 生成器
///
/// Clock を使って現在時刻ベースの ULID を生成します。
/// これにより、テスト時に FixedClock を使って決定的な ID を生成できます。
pub struct UlidGenerator<C> {
    clock: C,
}

impl<C: Clock> UlidGenerator<C> {
    /// 新しい UlidGenerator を作成
    pub fn new(clock: C) -> Self {
        Self { clock }
    }
}

impl<C: Clock> IdGenerator for UlidGenerator<C> {
    fn generate_job_id(&self) -> JobId {
        let timestamp_ms = self.clock.now().timestamp_millis() as u64;
        let ulid = Ulid::from_parts(timestamp_ms, rand::random());
        JobId::from(ulid)
    }

    fn generate_task_id(&self) -> TaskId {
        let timestamp_ms = self.clock.now().timestamp_millis() as u64;
        let ulid = Ulid::from_parts(timestamp_ms, rand::random());
        TaskId::from(ulid)
    }

    fn generate_attempt_id(&self) -> AttemptId {
        let timestamp_ms = self.clock.now().timestamp_millis() as u64;
        let ulid = Ulid::from_parts(timestamp_ms, rand::random());
        AttemptId::from(ulid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{FixedClock, SystemClock};
    use chrono::{TimeZone, Utc};

    #[test]
    fn ulid_generator_generates_unique_ids() {
        let id_gen = UlidGenerator::new(SystemClock);

        let id1 = id_gen.generate_job_id();
        let id2 = id_gen.generate_job_id();
        let id3 = id_gen.generate_job_id();

        // 各 ID が一意であることを確認
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn ulid_generator_with_fixed_clock_is_deterministic() {
        let fixed_time = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FixedClock::new(fixed_time);
        let id_gen = UlidGenerator::new(clock);

        let id1 = id_gen.generate_job_id();
        let id2 = id_gen.generate_job_id();

        // FixedClock を使っても、ランダム部分があるので ID は異なる
        assert_ne!(id1, id2);

        // ただし、timestamp 部分は同じはず
        let timestamp1 = (id1.as_ulid().0 >> 80) as u64;
        let timestamp2 = (id2.as_ulid().0 >> 80) as u64;
        assert_eq!(timestamp1, timestamp2);
        assert_eq!(timestamp1, fixed_time.timestamp_millis() as u64);
    }

    #[test]
    fn different_id_types_are_generated() {
        let id_gen = UlidGenerator::new(SystemClock);

        let job_id = id_gen.generate_job_id();
        let task_id = id_gen.generate_task_id();
        let attempt_id = id_gen.generate_attempt_id();

        // 型が異なることを確認（コンパイル時チェック）
        // let _: JobId = task_id; // <- これはコンパイルエラー

        // Display のプレフィックスが異なることを確認
        assert!(job_id.to_string().starts_with("job-"));
        assert!(task_id.to_string().starts_with("task-"));
        assert!(attempt_id.to_string().starts_with("attempt-"));
    }
}
