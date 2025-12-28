# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Weaver** is a Rust-based task execution engine that accepts a set of tasks, executes them with automatic retry/recovery, and returns comprehensive results including success/failure/incomplete states with full execution history.

**Core Philosophy:**
- Automatically retry failed tasks with backoff strategies
- Detect and stop when stuck (budget exhaustion, dependency cycles)
- Maintain full observability (every attempt, decision, and outcome is recorded)
- Support cancellation and status queries during execution
- Designed for learning Rust async patterns, ownership/lifetimes, and functional programming concepts

This is a **v1 implementation** focused on single-process execution with in-memory state. The architecture is intentionally designed to support future extensions (persistence, distributed workers, DAG optimization).

## Architecture

### Crate Structure

This is a Cargo workspace with two crates:

- **weaver-core**: Core domain model, execution primitives, and abstractions
- **weaver-cli**: CLI application and example implementations

### Domain Model (`weaver-core/src/domain/`)

The domain model uses strongly-typed IDs and algebraic data types to prevent type confusion and ensure exhaustive pattern matching:

- **IDs** (`ids.rs`): Newtype wrappers around `u64` for `JobId`, `TaskId`, and `AttemptId` to prevent mixing different identifier types
- **Specs** (`spec.rs`): Input types `JobSpec`, `TaskSpec`, and `Budget` - intentionally flexible with `serde_json::Value` fields for evolution without breaking changes
- **Outcomes** (`outcome.rs`): Common result format with three classifications:
  - `SUCCESS`: Forward progress (final or intermediate)
  - `FAILURE`: Recoverable failure (retry/alternatives/decomposition possible)
  - `BLOCKED`: Cannot proceed without additional info/prerequisites/intervention

### Key Abstractions (from requirements)

The following abstractions are described in the requirements but not all are implemented yet:

- **Job**: Unit of submission/cancellation/status/results (contains multiple tasks)
- **Task**: Minimum trackable unit (from input or dynamically added during execution)
- **Attempt**: Single execution try of a Task (records what was done and what happened)
- **Decision**: Record of "next action" choices (retry/decompose/add dependency/stop)
- **Artifact**: Execution outputs/references (files, URLs, stdout, JSON)
- **Dependency**: Inter-task dependencies (can be added during execution)
- **Budget**: Execution constraints (max attempts, deadlines, cost limits)

### Functional Programming Approach

The codebase intentionally adopts functional patterns as a learning exercise:

- **Pure decision logic**: Deciders should be pure functions (`current_state + observation → next_action`)
- **Isolated side effects**: Execution (Runners) contains side effects; everything else should be pure
- **Algebraic data types**: Use enums with exhaustive `match` to prevent logic gaps
- **Result/Option composition**: Prefer `map`/`and_then`/`fold` over imperative error handling

### Async Patterns (Tokio)

Critical non-functional requirements for async code:

- **Never `.await` while holding locks** - prevents deadlocks and latency spikes
- **Execute outside locks**: Clone/Arc data before async execution (e.g., `TaskEnvelope`)
- **Isolate blocking**: Use `spawn_blocking` for CPU-bound or blocking I/O
- **Cancellation-safe state transitions**: State must remain consistent if operations are cancelled

## Common Commands

### Build
```bash
cargo build
```

### Run Tests
```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p weaver-core
cargo test -p weaver-cli

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Run CLI Example
```bash
cargo run -p weaver-cli
```

### Check Code Without Building
```bash
cargo check
```

### Format Code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## Development Notes

### Current Implementation Status

The codebase is in active development (v1). Based on git status and compilation errors:

- ✅ Domain model (IDs, Specs, Outcomes) is complete
- ✅ Error types defined
- ✅ Worker abstraction skeleton exists
- ⚠️ Missing: `queue` module (referenced but not implemented)
- ⚠️ Missing: `runtime` module (referenced but not implemented)
- ⚠️ Main.rs references non-existent types: `ContentType`, `RetryPolicy`, `TaskEnvelope`, `TaskState`, `InMemoryQueue`, `Queue`, `HandlerRegistry`, `TaskHandler`, `execute_one`

### Design Constraints

When implementing new features, maintain these v1 boundaries:

1. **Single-process only**: No distributed coordination yet
2. **In-memory state**: Persistence will be pluggable later
3. **Replaceable execution**: Queue/Worker/Scheduler mechanisms should be swappable
4. **Status & Cancel**: Must support `get_status()` and `cancel_job()` operations
5. **No infinite loops**: All retry/backoff must respect budgets (max attempts, deadlines)

### Ownership & Lifetime Patterns

- **Long-lived data** (e.g., `TaskRecord`): Store in persistent collections
- **Short-lived data** (e.g., execution temporaries): Keep separate from records
- **No references in queues**: Store `TaskId` instead; keep entities in separate maps
- **Shared state**: Use `Arc` for immutable sharing, `Mutex`/`RwLock` for mutable (or prefer message-passing)

### Required Behavior: Automatic Progress

The system must automatically:

1. **Retry**: On `FAILURE`, retry with backoff until `max_attempts` or deadline
2. **Decompose**: Break abstract/large tasks into executable units (adds child tasks)
3. **Adapt**: Observe results (stdout/stderr/responses) and adjust next actions
4. **Stop when stuck**: Detect no-progress conditions:
   - All tasks BLOCKED with no RUNNABLE tasks remaining
   - Budget exhausted (attempts/time/cost)
   - Dependency cycles

### API Requirements (Minimum)

Future API interface must support:

```rust
submit_job(JobSpec) -> JobId
get_status(JobId) -> JobStatus
cancel_job(JobId) -> CancelAck
get_result(JobId) -> JobResult  // Includes partial results if incomplete
```

### Observability Requirements

All execution history must be preserved for explanation:

- **Attempt history**: What action was taken, what was observed, what outcome resulted
- **Decision history**: What policy was used, what data informed the decision, what change was made
- **Budget tracking**: Which constraint caused termination (if any)

This enables answering "why did this succeed/fail/stop?" after the fact.

## Language & Edition

- **Language**: Rust
- **Edition**: 2024 (non-standard; may be a typo for 2021)
- **Async Runtime**: Tokio with `rt-multi-thread`, `macros`, `time`, `sync` features

## Dependencies

Key external dependencies:
- `tokio`: Async runtime
- `serde` / `serde_json`: Serialization with derive macros
- `thiserror`: Error type derivation
- `async-trait`: Trait async methods

## Development Documentation

The `dev/docs/` directory contains critical development documentation:

### Requirements Documents (Japanese)

**Important**: Requirements are stored with date prefixes (YYYY_MM_DD). **Always use the file with the newest date** as the current authoritative specification. Older files are kept for historical reference.

**Current (Latest): `dev/docs/requirements/2025_12_27_weaver_requirements.md`**
- Job-level abstraction (multiple tasks per job)
- Decision/Attempt/Artifact model
- Automatic decomposition and dependency discovery
- Budget constraints (attempts, deadlines, cost limits)
- Stuck detection (no progress, dependency cycles)
- Cancel and status APIs
- Functional programming approach (pure deciders, isolated effects)
- Full audit trail requirements

**Historical: `dev/docs/requirements/2025_12_26_weaver_requirements.md`**
- Initial v1 requirements
- In-memory task queue design
- TaskEnvelope + TaskHandler pattern
- Retry policy specifications (max 5 attempts, backoff)
- TaskState machine (Queued → Running → Succeeded/RetryScheduled/Dead)
- Queue data structures (HashMap for records, VecDeque for ready queue, scheduled queue)
- Tokio async guardrails (never `.await` while holding locks)
- Observability requirements (get_status, counts_by_state)

**Note**: These documents are in Japanese and contain the authoritative specifications. Always consult the latest requirements document when implementing new features or making architectural decisions.

### Architecture Decision Records (ADR)

- **`dev/docs/adr/`**: Will contain Architecture Decision Records documenting significant architectural choices, trade-offs, and rationale
  - ADRs provide historical context for why specific design decisions were made
  - Consult existing ADRs before making changes that might conflict with documented decisions
