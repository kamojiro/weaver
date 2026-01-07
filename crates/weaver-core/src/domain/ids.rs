//! Domain identifiers (strongly-typed IDs).
//!
//! # v2: ULID ベースの ID + ジェネリック実装
//! v2 では ULID (Universally Unique Lexicographically Sortable Identifier) を使用します。
//! さらに、Phantom type パターンを使ってコードの重複を排除しています。
//!
//! ## ULID の特性
//! - **時刻でソート可能**: timestamp が先頭にあるため、生成順序でソートできる
//! - **分散生成可能**: 調整なしで複数ノードで生成できる
//! - **UUID互換**: 128-bit で UUID と同じサイズ
//!
//! ## Phantom Type パターン
//! `Id<T>` というジェネリック型で共通実装を提供しつつ、
//! `T` は実行時には使わない（PhantomData）マーカー型として、
//! コンパイル時の型安全性を提供します。
//!
//! ## なぜこのパターンを使うのか？
//! - コードの重複を排除（DRY原則）
//! - 型安全性を維持（JobId と TaskId は混同できない）
//! - 一貫性のある実装（バグが入りにくい）

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use ulid::Ulid;

/// IdMarker は各 ID 型のマーカー trait
///
/// Display で使うプレフィックス（"job-", "task-", "attempt-"）を提供します。
pub trait IdMarker: Send + Sync + 'static {
    /// Display で使うプレフィックス（例: "job-", "task-"）
    fn prefix() -> &'static str;
}

/// ジェネリック ID 型
///
/// `T` は PhantomData で、実行時にはメモリを消費しませんが、
/// コンパイル時に型安全性を提供します。
///
/// # 例
/// ```ignore
/// let job_id: JobId = Id::from(Ulid::new());
/// let task_id: TaskId = Id::from(Ulid::new());
/// // job_id と task_id は異なる型なので、混同できない
/// ```
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id<T: IdMarker> {
    ulid: Ulid,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

impl<T: IdMarker> Id<T> {
    /// ULID から Id を作成
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self {
            ulid,
            _marker: PhantomData,
        }
    }

    /// テスト用: u128 から Id を作成（v1互換）
    ///
    /// # Deprecation
    /// このメソッドは v1 コードとの互換性のためにのみ提供されています。
    /// v2 コードでは `from_ulid()` または `IdGenerator` を使用してください。
    #[deprecated(note = "Use from_ulid() or IdGenerator instead. This is for v1 compatibility only.")]
    pub fn new(value: u128) -> Self {
        // u128 を ULID bytes として解釈
        let bytes = value.to_be_bytes();
        let ulid = Ulid::from_bytes(bytes);
        Self::from_ulid(ulid)
    }

    /// 内部の ULID を取得
    pub fn as_ulid(&self) -> Ulid {
        self.ulid
    }

    /// テスト用: u64 として取得（v1互換）
    #[deprecated(note = "This is for v1 compatibility only.")]
    pub fn as_u64(&self) -> u64 {
        // ULID の最後 8 バイトを u64 として返す
        let bytes = self.ulid.to_bytes();
        u64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ])
    }
}

impl<T: IdMarker> From<Ulid> for Id<T> {
    fn from(ulid: Ulid) -> Self {
        Self::from_ulid(ulid)
    }
}

impl<T: IdMarker> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", T::prefix(), self.ulid)
    }
}

// ========================================
// マーカー型の定義
// ========================================

/// Job のマーカー型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Job {}

impl IdMarker for Job {
    fn prefix() -> &'static str {
        "job-"
    }
}

/// Task のマーカー型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Task {}

impl IdMarker for Task {
    fn prefix() -> &'static str {
        "task-"
    }
}

/// Attempt のマーカー型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Attempt {}

impl IdMarker for Attempt {
    fn prefix() -> &'static str {
        "attempt-"
    }
}

// ========================================
// Type Alias（使いやすさのため）
// ========================================

/// Identifier of a Job (submit/status/cancel/result unit).
pub type JobId = Id<Job>;

/// Identifier of a Task (trackable unit within a Job).
pub type TaskId = Id<Task>;

/// Identifier of an Attempt (one execution try of a Task).
pub type AttemptId = Id<Attempt>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_types() {
        let ulid1 = Ulid::new();
        let ulid2 = Ulid::new();
        let ulid3 = Ulid::new();

        let job = JobId::from_ulid(ulid1);
        let task = TaskId::from_ulid(ulid2);
        let attempt = AttemptId::from_ulid(ulid3);

        // 型が異なることを確認（as_ulid で取得できる）
        assert_eq!(job.as_ulid(), ulid1);
        assert_eq!(task.as_ulid(), ulid2);
        assert_eq!(attempt.as_ulid(), ulid3);

        // Display のプレフィックスが正しいことを確認
        assert!(job.to_string().starts_with("job-"));
        assert!(task.to_string().starts_with("task-"));
        assert!(attempt.to_string().starts_with("attempt-"));

        // The whole point: you can't accidentally mix these types.
        // (This is a compile-time property, so we just keep it as a comment.)
        // let _: JobId = task; // <- does not compile
    }

    #[test]
    fn ulid_ids_are_sortable() {
        // ULID は時刻ベースなので、生成順序でソート可能
        let id1 = JobId::from_ulid(Ulid::new());
        std::thread::sleep(std::time::Duration::from_millis(2)); // 時刻が進むのを待つ
        let id2 = JobId::from_ulid(Ulid::new());
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id3 = JobId::from_ulid(Ulid::new());

        // 生成順序でソートされることを確認
        assert!(id1 < id2);
        assert!(id2 < id3);
        assert!(id1 < id3);
    }

    #[test]
    fn ulid_ids_can_be_serialized() {
        let job_id = JobId::from_ulid(Ulid::new());

        // Serialize/Deserialize のラウンドトリップテスト
        let serialized = serde_json::to_string(&job_id).unwrap();
        let deserialized: JobId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(job_id, deserialized);
    }

    #[test]
    fn from_trait_works() {
        let ulid = Ulid::new();

        // From<Ulid> トレイトが動作することを確認
        let job_id: JobId = ulid.into();
        assert_eq!(job_id.as_ulid(), ulid);

        let task_id: TaskId = ulid.into();
        assert_eq!(task_id.as_ulid(), ulid);

        let attempt_id: AttemptId = ulid.into();
        assert_eq!(attempt_id.as_ulid(), ulid);
    }

    #[test]
    fn phantom_data_does_not_consume_memory() {
        // PhantomData はメモリを消費しないことを確認
        use std::mem::size_of;

        // Id<T> のサイズは Ulid と同じ（16 bytes）
        assert_eq!(size_of::<JobId>(), size_of::<Ulid>());
        assert_eq!(size_of::<TaskId>(), size_of::<Ulid>());
        assert_eq!(size_of::<AttemptId>(), size_of::<Ulid>());
        assert_eq!(size_of::<Ulid>(), 16); // ULID は 128-bit = 16 bytes
    }
}
