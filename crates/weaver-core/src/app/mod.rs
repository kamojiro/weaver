//! App - アプリケーション層
//!
//! このモジュールは、ports を組み合わせてアプリケーションロジックを実装します。
//!
//! # 主要コンポーネント
//! - **AppBuilder**: アプリケーションの構築とワイヤリング
//! - **Runtime**: 型付き Task API の表面
//! - **WorkerLoop**: タスク実行ループ（pop→claim→handle→decide→complete）
//! - **PublisherLoop**: Outbox イベントの配送
//! - **ReaperLoop**: Lease 期限切れの回収
//! - **GCLoop**: Artifact のガベージコレクション

pub mod builder;
pub mod runtime;
pub mod worker_loop;
pub mod publisher_loop;
pub mod reaper_loop;
pub mod gc_loop;
pub mod status;

// 主要な型を再エクスポート
pub use self::builder::AppBuilder;
pub use self::runtime::Runtime;
pub use self::worker_loop::WorkerLoop;
pub use self::publisher_loop::PublisherLoop;
pub use self::reaper_loop::ReaperLoop;
pub use self::gc_loop::GCLoop;
