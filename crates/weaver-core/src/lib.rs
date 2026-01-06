//! weaver-core
//!
//! Core building blocks for the Weaver runtime.
//!
//! # v2 モジュール構成（移行中）
//! - **domain**: ドメインモデル（ids, task_type, envelope, budget, outcome, decision, state, errors, events）
//! - **ports**: 抽象化レイヤー（TaskStore, DeliveryQueue, ArtifactStore, Decider, など）
//! - **app**: アプリケーションロジック（builder, runtime, worker_loop, publisher_loop, など）
//! - **typed**: 型付き Task API（Task trait, Handler trait, TypedRegistry, PayloadCodec）
//! - **impls**: 実装（InMemoryDeliveryQueue など開発用）
//!
//! # v1 互換モジュール（deprecated）
//! - queue: Queue trait + in-memory implementation → ports/delivery_queue + impls/inmem_delivery に移行
//! - runtime: handler registry → app/runtime に移行
//! - worker: worker 実行ロジック → app/worker_loop に移行
//! - observability: status views → app/status に移行
//! - error: エラー型 → domain/errors に移行

// v2 の新しいモジュール
pub mod domain;
pub mod ports;
pub mod app;
pub mod typed;
pub mod impls;

// v1 の既存モジュール（deprecated、互換性維持）
#[deprecated(
    note = "Use `domain::errors` instead. This module will be removed in a future version."
)]
pub mod error;

#[deprecated(
    note = "Use `app::worker_loop` instead. This module will be removed in a future version."
)]
pub mod worker;

#[deprecated(
    note = "Use `ports::delivery_queue` and `impls::inmem_delivery` instead. This module will be removed in a future version."
)]
pub mod queue;

#[deprecated(
    note = "Use `app::runtime` instead. This module will be removed in a future version."
)]
pub mod runtime;

#[deprecated(
    note = "Use `app::status` instead. This module will be removed in a future version."
)]
pub mod observability;
