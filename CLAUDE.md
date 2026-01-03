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

This is a **v1 implementation** focused on single-process execution with in-memory state.

## Learning Mode (CRITICAL)

**IMPORTANT**: This project is being developed as a Rust learning exercise.

**For Claude Code:**
- The user is actively learning Rust, async patterns, and functional programming
- **Do NOT automatically implement features** unless explicitly requested
- **Instead, provide:**
  - Implementation hints and guidance
  - TODO(human) markers in code for learning opportunities
  - "Learn by Doing" format requests for key implementation tasks
  - Code reviews and explanations after user implements features

**Language Preference:**
- **日本語で回答・解説する**: ユーザーは日本語話者なので、説明、ヒント、コードレビューは日本語で提供すること
- Code comments and documentation should remain in English (as per project convention)
- Technical discussions and explanations should be in Japanese

**Learning task management:**
- `dev/learning/tasks.md` - Master task list for all phases (check here for what needs to be done)
- `dev/learning/learning_YYYY_MM_DD.md` - Daily implementation logs (detailed records with code and insights)
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

### 3. V1 Boundaries
1. **Single-process only**: No distributed coordination yet
2. **In-memory state**: Persistence will be pluggable later
3. **Replaceable execution**: Queue/Worker/Scheduler mechanisms should be swappable
4. **Status & Cancel**: Must support `get_status()` and `cancel_job()` operations
5. **No infinite loops**: All retry/backoff must respect budgets (max attempts, deadlines)

## Documentation References

### Requirements (Authoritative Specification)

**Important**: Requirements are stored with date prefixes (YYYY_MM_DD). **Always use the file with the newest date** as the current authoritative specification.

**Current (Latest): `dev/docs/requirements/2025_12_27_weaver_requirements.md`**
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
