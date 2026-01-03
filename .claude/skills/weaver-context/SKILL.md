---
description: Detailed Weaver architecture, requirements, and development context
---

# Weaver Context

This skill provides detailed architecture, requirements reference, and development guidelines for the Weaver project.

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

The codebase is in active development (v1). Current state as of 2026-01-03:

**✅ Implemented:**
- **Phase 1**: Basic task execution and retry (Complete 2025-12-28)
  - Domain model (IDs, Specs, Outcomes, TaskEnvelope, TaskType)
  - Error types and WeaverError
  - Queue trait + InMemoryQueue implementation
  - TaskLease, TaskRecord, TaskState
  - RetryPolicy with backoff support
  - HandlerRegistry + Runtime for task execution
  - Worker/WorkerGroup for concurrent task processing
- **Phase 2**: Job-level abstraction (Complete 2025-12-29)
  - submit_job API
  - Multiple tasks per job
  - Job state aggregation
- **Phase 3**: Attempt/Decision recording (Complete 2025-12-30)
  - AttemptRecord and DecisionRecord
  - Full audit trail
- **Phase 4-1**: Decider integration (Complete 2026-01-02)
  - Handler → Outcome → Decider → Decision flow
  - Pure function decision logic
- **Phase 4**: Task decomposition (Complete 2026-01-03)
  - Child task creation from Handler proposals
  - Parent-child relationship tracking
  - Decomposed task state
  - Lock minimization patterns (ADR-0003 compliant)

**📚 Learning Tasks (See `dev/learning/` for details):**
- Phase 5: Dependency management and resolution
- Phase 6: Budget constraints and stuck detection
- Phase 7: Complete API (get_status, cancel_job, get_result)
- Phase 8: Artifact storage and retrieval

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

### Testing Strategy

Adopt a layered testing approach:

1. **Unit Tests**: Test individual components in isolation
   - Test pure functions (e.g., Decider logic, RetryPolicy calculations)
   - Test single methods with mocked dependencies
   - Fast feedback, easy to debug

2. **Integration Tests**: Test component interactions
   - Test data flow between components (e.g., Worker → Decider → Queue)
   - Verify state transitions across boundaries
   - Catch interface mismatches and integration bugs

3. **End-to-End Tests**: Test complete scenarios
   - Test full user workflows (e.g., job submission → decomposition → completion)
   - Verify system behavior under realistic conditions
   - Often reveal bugs missed by lower-level tests

**Key Insight**: Integration and E2E tests frequently discover bugs that unit tests miss, such as:
- Incorrect data flow between components (e.g., submit_job using wrong field)
- Missing logic branches (e.g., Worker not checking child_tasks on Success)
- State management issues across component boundaries

### Debugging Best Practices

When investigating bugs:

1. **Read error messages carefully**: Extract specific details that reveal the root cause
   - Example: "handler not found for task_type=parent task" → wrong field being used as task_type

2. **Trace data flow**: Follow data through the system from input to output
   - Example: JobSpec → submit_job → TaskEnvelope → Worker → Handler
   - Identify where data transformation goes wrong

3. **Fix incrementally**: Address one issue at a time, verify with tests
   - Avoid batching multiple fixes without verification
   - Each fix should make tests progress toward success

4. **Document discoveries**: Record useful patterns and gotchas
   - Create files in `dev/docs/tips/` for reusable knowledge
   - Example: Lock minimization pattern, common pitfalls, debugging techniques

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

## Requirements Documents

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

- **`dev/docs/adr/`**: Contains Architecture Decision Records documenting significant architectural choices, trade-offs, and rationale
  - ADRs provide historical context for why specific design decisions were made
  - Consult existing ADRs before making changes that might conflict with documented decisions

### Learning Tasks

- **`dev/learning/`**: Learning task management and progress tracking
  - **File Format**: Date-prefixed files (`learning_YYYY_MM_DD.md`) - the newest date is always the current task list
  - **Purpose**: Track learning progress, implementation tasks, and study notes
  - **Content**: Each file contains:
    - Task checklist (completed/incomplete)
    - Learning notes and discoveries
    - Implementation progress and next steps

## Code Review Checklist

When reviewing implementations, verify:

### Async Safety
- [ ] No `.await` while holding locks (ADR-0003)
- [ ] Clone/Arc data before async operations
- [ ] Cancellation-safe state transitions

### Ownership & Lifetimes
- [ ] No unnecessary clones (use references where safe)
- [ ] Clear ownership transfer semantics
- [ ] Lifetime annotations only where necessary

### Functional Patterns
- [ ] Pure decision logic (no side effects in deciders)
- [ ] Exhaustive pattern matching on enums
- [ ] Result/Option composition instead of nested if-let

### Error Handling
- [ ] WeaverError with proper context
- [ ] `map_err` for error conversion
- [ ] `?` operator for propagation

### Observability
- [ ] All attempts recorded
- [ ] All decisions logged
- [ ] Budget tracking maintained

### Testing
- [ ] Unit tests for pure logic and isolated components
- [ ] Integration tests for component interactions
- [ ] E2E tests for complete user scenarios (when adding major features)
- [ ] Tests verify both success and failure paths
- [ ] Error messages are clear and actionable

### Documentation
- [ ] Complex patterns documented in `dev/docs/tips/` (if reusable)
- [ ] ADR-0003 compliance verified for async code
- [ ] Learning insights recorded in daily log (`dev/learning/learning_YYYY_MM_DD.md`)
