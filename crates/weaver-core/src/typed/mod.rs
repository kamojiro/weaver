//! Typed - 型付き Task API
//!
//! このモジュールは task_type の typo を型で排除し、
//! Handler との対応付けを静的に保証します。
//!
//! # 二層構造
//! - **表層（Typed）**: `Task` trait, `Handler<T>` trait - 型安全
//! - **内部（Dyn）**: `DynHandler` trait - object-safe, type erasure
//!
//! # 実装予定
//! - **PR-3**: Typed Task API の実装（TODO(human)）

pub mod task;
pub mod handler;
pub mod registry;
pub mod codec;

// 主要な trait/型 を再エクスポート
pub use self::task::Task;
pub use self::handler::{Handler, DynHandler};
pub use self::registry::{TypedRegistry, RegistryError};
pub use self::codec::{PayloadCodec, CodecError};
