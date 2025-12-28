//! weaver-core
//!
//! Core building blocks for the Weaver runtime.
//!
//! This crate is intentionally split into small modules for learning:
//! - domain: ids, states, envelopes, records, retry policy
//! - queue: Queue trait + in-memory implementation
//! - runtime: handler registry and execution helpers
//! - observability: status views and state counts
//! - error: crate-level error types

pub mod domain;
pub mod error;
pub mod worker;
pub mod queue;
pub mod runtime;
pub mod observability;
