//! Ports - 抽象化レイヤー
//!
//! このモジュールは Hexagonal Architecture の「ポート」を定義します。
//! 各 trait は外部システム（PostgreSQL, Redis, Blob storage など）への
//! インターフェースを提供し、実装の詳細を隠蔽します。
//!
//! # v2 の設計原則
//! - PostgreSQL が source of truth（正本）
//! - Redis は配送キュー（task_id のみ）
//! - Blob storage は巨大データ（artifact）の保存先

pub mod task_store;
pub mod delivery_queue;
pub mod artifact_store;
pub mod decider;
pub mod dispatch;
pub mod repair_hint;
pub mod clock;
pub mod id_generator;
pub mod event_sink;

// 主要な trait を再エクスポート
pub use self::task_store::TaskStore;
pub use self::delivery_queue::{DeliveryQueue, QueueError};
pub use self::artifact_store::ArtifactStore;
pub use self::decider::Decider;
pub use self::dispatch::DispatchStrategy;
pub use self::repair_hint::RepairHintGenerator;
pub use self::clock::{Clock, SystemClock, FixedClock};
pub use self::id_generator::{IdGenerator, UlidGenerator};
pub use self::event_sink::EventSink;
