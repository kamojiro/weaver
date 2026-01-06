//! Impls - 実装（開発用・テスト用）
//!
//! このモジュールには ports の実装を含めます。
//!
//! # 含まれる実装
//! - **InMemoryDeliveryQueue**: 開発用の配送キュー
//! - （将来）InMemoryTaskStore: テスト用の正本
//!
//! # 本番用実装
//! 本番用の実装は別クレートに配置します：
//! - `weaver-pg`: PostgresTaskStore
//! - `weaver-redis`: RedisDeliveryQueue
//! - `weaver-blob`: MinIO/S3/LocalArtifactStore

pub mod inmem_delivery;

// 主要な型を再エクスポート
pub use self::inmem_delivery::InMemoryDeliveryQueue;
