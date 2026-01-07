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

---

## 🔄 Project Status: v1 → v2 Migration

**v1 (COMPLETED)**: Single-process, in-memory learning prototype
- **Purpose**: Learn Rust fundamentals (async, ownership, functional patterns)
- **Status**: Phase 1-4 complete (basic execution through task decomposition)
- **Archive**: See `dev/learning/tasks_v1/`

**v2 (CURRENT)**: Production-ready distributed task execution system
- **Purpose**: Build a real-world task orchestration engine
- **Architecture**: PostgreSQL (source of truth) + Redis (delivery) + Blob storage
- **Key Patterns**: Outbox pattern, Lease-based execution, Ports & Adapters
- **Timeline**: ~2 weeks (14 PRs across 2 phases)
- **Tasks**: See `dev/learning/tasks.md`

**What's Different in v2:**
- **PostgreSQL as source of truth**: All state, history, dependencies, and outbox events
- **Redis for delivery**: Task IDs only (no state/payload in Redis)
- **Typed Task API**: Compile-time task_type validation with runtime registration check
- **Outbox pattern**: Transactional consistency between state changes and delivery
- **Lease-based execution**: At-least-once delivery with visibility timeout
- **Artifact management**: Blob storage (MinIO/S3/Local) with TTL and GC
- **Repair/Regenerate**: Automatic recovery from decode failures

## Learning Mode (CRITICAL)

**IMPORTANT**: This project is being developed as a Rust learning exercise.

**For Claude Code:**
- The user is actively learning Rust, async patterns, and functional programming
- **Do NOT automatically implement features** unless explicitly requested
- **CRITICAL: Let the user implement code themselves**
  - **Default behavior**: Provide TODO(human) and "Learn by Doing" requests, then WAIT for user implementation
  - **Sample implementations are OPTIONAL**: Provide sample/example implementations (1 out of 3 similar items) only when helpful for demonstration; otherwise, skip and let user implement all parts
  - **Only implement directly when**: User explicitly requests "please implement this" or "do this for me"
- **Instead of implementing, provide:**
  - Clear task descriptions and requirements
  - TODO(human) markers in code for learning opportunities
  - "Learn by Doing" format requests for key implementation tasks
  - (Optional) Sample implementation if it helps clarify the pattern
  - Code reviews and explanations after user implements features

- **CRITICAL: TODO(human) comments in source code**
  - **DO NOT write complete implementations in TODO comments** - this defeats the learning purpose
  - **Instead, write hints and guidance:**
    - What the code should accomplish (purpose)
    - Key considerations or constraints
    - Suggested approach or pattern name
    - References to learning docs for full examples
  - **Complete implementations belong in:**
    - `dev/learning/learning_YYYY_MM_DD.md` (detailed hints in collapsible sections)
    - NOT in source code comments

**Language Preference:**
- **日本語で回答・解説する**: ユーザーは日本語話者なので、説明、ヒント、コードレビューは日本語で提供すること
- Code comments and documentation should remain in English (as per project convention)
- Technical discussions and explanations should be in Japanese

**Learning task management:**
- `dev/learning/tasks.md` - **v2** master task list (14 PRs, Week 1-2)
- `dev/learning/tasks_v1/tasks.md` - v1 task archive (completed)
- `dev/learning/learning_YYYY_MM_DD.md` - Daily implementation logs (v2)
- `dev/learning/tasks_v1/learning_YYYY_MM_DD.md` - v1 implementation logs (archive)
- `dev/learning/README.md` - Usage guide for the learning directory

**IMPORTANT for Claude Code:**
- Tasks in `dev/learning/` are **intentionally left for the learner to implement**
- **DO NOT automatically implement** these tasks unless explicitly requested by the user

**Learning Flow Guidelines:**
When working with the user on implementation tasks:
1. **Provide context first**: Explain what the task accomplishes and why it matters
2. **Offer incremental hints**: Start with conceptual guidance, provide code examples only when needed
3. **Encourage questions**: Explicitly invite the user to ask questions at each stage
4. **Work collaboratively when requested**: If the user chooses to implement together (vs. alone), guide step-by-step
5. **Review and reflect**: After implementation, discuss what was learned and broader implications

**Testing Strategy:**
- **Unit tests**: Test individual components in isolation (e.g., add_child_tasks() logic)
- **Integration tests**: Test component interactions (e.g., Worker → Decider → Queue flow)
- **End-to-End tests**: Test complete user scenarios (e.g., job submission → decomposition → completion)
- Integration/E2E tests often reveal bugs that unit tests miss (e.g., Worker Success handling, data flow issues)

**Debugging Process:**
When helping debug issues:
1. **Read error messages carefully**: Extract specific details (e.g., "task_type=parent task" indicates wrong field usage)
2. **Trace data flow**: Follow data through the system (e.g., submit_job → Worker → complete)
3. **Incremental fixes**: Fix one issue at a time, verify with tests before proceeding
4. **Document discoveries**: Encourage documenting patterns in `dev/docs/tips/` for future reference

### Learning Task Documentation Template

**For complex implementation tasks (10+ lines, involving ownership/async/locks)**, create detailed task documentation in `dev/learning/learning_YYYY_MM_DD.md` BEFORE the user starts implementation.

**Template structure:**
```markdown
## ⏳ [Step/Phase Name]: [Task Title]（学習タスク）

### 📋 タスクの概要
[1-2 sentences describing what this task accomplishes]

### 🎯 学習目標
このタスクを通じて、以下の Rust の概念を実践的に学びます：
1. **[Concept]** - [Specific skills]

### 📝 実装すべき内容
**変更するファイル:** [List files]

### ✅ 機能要件
[Detailed requirements]

### 🚨 技術的制約（非常に重要）
[Critical constraints with examples]

### 💡 実装のヒント（質問があれば聞いてください）
[Hints in collapsible sections]

### 🔍 実装後の確認事項
[Checklist items]
```

## Absolute Constraints

When implementing new features, these constraints are **MANDATORY**:

### 1. Async Safety (ADR-0003)
- **Never `.await` while holding locks** - prevents deadlocks and latency spikes
- **Execute outside locks**: Clone/Arc data before async execution (e.g., `TaskEnvelope`)
- **Isolate blocking**: Use `spawn_blocking` for CPU-bound or blocking I/O
- **Cancellation-safe state transitions**: State must remain consistent if operations are cancelled

### 2. Functional Programming Patterns
- **Pure decision logic**: Deciders should be pure functions (`current_state + observation → next_action`)
- **Isolated side effects**: Execution (Runners) contains side effects; everything else should be pure
- **Algebraic data types**: Use enums with exhaustive `match` to prevent logic gaps
- **Result/Option composition**: Prefer `map`/`and_then`/`fold` over imperative error handling

### 3. V2 Design Invariants

**MUST maintain these invariants in v2 implementation:**

1. **PostgreSQL is source of truth**
   - Redis is for delivery only (task_id + lightweight meta)
   - Never store state/payload/envelope in Redis
   - All state must be reconstructible from PostgreSQL

2. **Lease authority is in PostgreSQL**
   - Redis pop is just a "candidate notification"
   - Execution authority is determined by `TaskStore.claim()` success only

3. **Outbox pattern is mandatory**
   - When a task becomes `ready`, MUST append `dispatch_task` to outbox in same TX
   - Never allow "ready but not dispatched" state (prevents lost tasks)

4. **Payload via artifact_ref**
   - No large data embedded in PG/Redis
   - Enforce size limits, force artifact storage for oversized payloads
   - Support TTL (expires_at) for automatic cleanup

5. **Dependencies fixed at creation**
   - v2: Dependencies are established at task creation time
   - No adding dependencies after task execution starts (or strong constraints if needed)
   - Simplifies `remaining_deps` ready-check logic

6. **Transaction boundaries are sacred**
   - State transitions (claim/complete/reap) + outbox generation happen in TaskStore TX
   - App layer never directly mutates state, only calls ports

**v1 Boundaries (archived for reference):**
1. ~~Single-process only~~ → v2: Distributed (PG/Redis/Workers)
2. ~~In-memory state~~ → v2: PostgreSQL persistence
3. ✅ Replaceable execution (still true: Queue/Worker swappable via ports)
4. ✅ Status & Cancel support (carried forward to v2)
5. ✅ No infinite loops (v2 adds max_repairs for decode failures)

## Documentation References

### Requirements (Authoritative Specification)

**Important**: Requirements are stored with date prefixes (YYYY_MM_DD). **Always use the file with the newest date** as the current authoritative specification.

**Current (Latest): `dev/docs/requirements/2026_01_03_weaver_requirements.md` (v2)**
- PostgreSQL as source of truth + Outbox pattern
- Redis delivery queue (task_id only)
- Typed Task API with startup validation
- Lease + visibility timeout (at-least-once delivery)
- Decode failure recovery (repair tasks + max_repairs)
- Artifact storage with TTL and GC
- Docker Compose full-stack deployment
- 2-week implementation plan (14 PRs)
- Module structure: domain / ports / app / typed / impls

**Historical: `dev/docs/requirements/2025_12_27_weaver_requirements.md` (v1)**
- Job-level abstraction (multiple tasks per job)
- Decision/Attempt/Artifact model
- Automatic decomposition and dependency discovery
- Budget constraints (attempts, deadlines, cost limits)
- Stuck detection (no progress, dependency cycles)
- Cancel and status APIs
- Functional programming approach (pure deciders, isolated effects)
- Full audit trail requirements

**Note**: These documents are in Japanese and contain the authoritative specifications.

### Architecture & Development Context

For detailed architecture, commands, implementation status, and code review checklists, use the **weaver-context** skill:

```
Use the weaver-context skill when you need:
- Architecture details (crate structure, domain model, key abstractions)
- Common commands (cargo build, test, clippy, etc.)
- Development notes (implementation status, design constraints)
- Requirements document details
- Code review checklist
```

### Architecture Decision Records (ADR)

- **`dev/docs/adr/`**: Contains Architecture Decision Records documenting significant architectural choices
  - Consult existing ADRs before making changes that might conflict with documented decisions
  - ADR-0003: Never `.await` while holding locks

## Quick Commands

```bash
cargo check          # Fast compilation check
cargo test           # Run all tests
cargo test -p NAME   # Run tests for specific crate
cargo clippy         # Lint
cargo fmt            # Format
```

For more commands and options, use the **weaver-context** skill.
