# Algebraic Effects と Weaver の関係性

**日付**: 2026-01-01
**ステータス**: 探索的考察（実装予定なし）
**目的**: Weaver の設計と algebraic effects の概念的な関連性を理解・記録する

## 背景

Job と Task の関係を理解する過程で、Weaver の全体的な設計が algebraic effects のパターンと類似していることに気づいた。この文書では、その関連性を分析し、将来的な設計の可能性を探索する。

## Algebraic Effects の基本概念

Algebraic effects は、副作用を宣言的に扱うためのプログラミング概念：

1. **Effect**: 副作用の宣言的な記述（`perform RetryEffect(delay)`）
2. **Handler**: Effect の実際の実装（effect を intercept して実行）
3. **Resume/Continuation**: Handler が計算を再開する仕組み
4. **純粋性**: Effect を "perform" する側は純粋関数のまま

**典型的なパターン:**
```
Pure Computation → perform Effect → Handler intercepts → resume with result
```

## Weaver における Algebraic Effects 的な構造

### 1. Effect Declaration: `Decision` enum

**ファイル**: `crates/weaver-core/src/domain/decision.rs:16-27`

```rust
enum Decision {
    Retry { delay: Duration, reason: String },
    MarkDead { reason: String },
    // 将来: Decompose, AddDependency, RequestInput...
}
```

これは algebraic effects の "effect operations" に相当：
- `Retry` = `perform Retry(delay, reason)`
- `MarkDead` = `perform MarkDead(reason)`

### 2. Perform: `Decider` trait

**ファイル**: `crates/weaver-core/src/domain/decision.rs:36-46`

```rust
trait Decider {
    fn decide(&self, task: &TaskRecord, outcome: &Outcome) -> Decision;
}
```

**特徴:**
- **純粋関数**: 状態 + 観測 → 次のアクション（副作用の記述）
- 実際に副作用を実行せず、`Decision` を返すのみ
- これは effect を "perform" するパターン

### 3. Handler: Worker

**ファイル**: `crates/weaver-core/src/queue/worker.rs`（実装中）

Worker が `Decision` を受け取って実際に実行：

```rust
match decision {
    Decision::Retry { delay, .. } => schedule_retry(task, delay),
    Decision::MarkDead { .. } => mark_task_dead(task),
}
```

これは effect handler の役割：
- Effect (Decision) を intercept
- 実際の副作用を実行
- （将来的には）resumption も含む

### 4. Effect Trace: AttemptRecord / DecisionRecord

**ファイル**: `crates/weaver-core/src/domain/attempt.rs`

```rust
struct AttemptRecord {
    action: Value,              // 何を perform したか
    observation: Vec<Artifact>, // Handler が何を返したか
    outcome: Outcome,           // 結果
}

struct DecisionRecord {
    trigger: Value,   // 何が perform をトリガーしたか
    policy: String,   // どの handler/policy を使ったか
    decision: String, // 何を perform したか
}
```

これは **effect tracing/logging** に相当。Algebraic effects では handler invocation を記録できるが、Weaver では明示的に Record として保存している。

## 要件との対応

**要件ドキュメント**: `dev/docs/requirements/2025_12_27_weaver_requirements.md:236-244`

> ### 関数型っぽさを活かす（要求として）
>
> * "判断（Decider）" は **できるだけ純粋関数**に寄せる
>   * 入力: 現在状態 + 観測/結果
>   * 出力: 次の操作（再試行/分解/依存追加/停止）＝**副作用の指示**
> * "実行（Runner）" に副作用を閉じ込める

これは algebraic effects の核心的なパターンと完全に一致している。

## 対応表

| Algebraic Effects 概念 | Weaver における実装 | ファイル |
|----------------------|-------------------|----------|
| Effect Declaration | `Decision` enum | decision.rs:16-27 |
| Perform | `Decider::decide()` 返り値 | decision.rs:36-46 |
| Handler | Worker の Decision dispatch | queue/worker.rs |
| Pure Computation | `Decider` trait impl | decision.rs:75-96 |
| Resume/Continue | Task の再スケジュール | queue/ |
| Effect Trace | AttemptRecord, DecisionRecord | attempt.rs |

## 将来的な拡張可能性

### Option A: 既存設計の自然な拡張（推奨）

`Decision` enum を拡張して、より多様な effect を表現：

```rust
enum Decision {
    Retry { delay: Duration, reason: String },
    MarkDead { reason: String },

    // Phase 4-2+: より複雑な effect
    Decompose {
        subtasks: Vec<TaskSpec>,
        composition_strategy: CompositionStrategy,
    },
    AddDependency {
        depends_on: TaskId,
        dependency_type: DependencyType,
    },
    RequestInput {
        prompt: String,
        schema: serde_json::Value,
    },
    Delegate {
        to_agent: AgentId,
        context: serde_json::Value,
    },
}
```

Worker を汎用的な "Decision handler" として実装：

```rust
impl Worker {
    async fn handle_decision(&self, task_id: TaskId, decision: Decision) {
        match decision {
            Decision::Retry { delay, .. } => {
                self.schedule_retry(task_id, delay).await
            }
            Decision::Decompose { subtasks, strategy } => {
                self.create_subtasks(task_id, subtasks, strategy).await
            }
            Decision::RequestInput { prompt, schema } => {
                self.wait_for_input(task_id, prompt, schema).await
            }
            // ...
        }
    }
}
```

**利点:**
- 既存の設計パターンを活かせる
- 段階的に拡張可能
- Rust の型システムと相性が良い（exhaustive matching）

### Option B: より明示的な Effect System

OCaml や Koka のような型レベル effect tracking：

```rust
// Effect trait with associated types
trait Effect {
    type Output;
    type Error;
}

struct RetryEffect {
    delay: Duration,
    reason: String,
}

impl Effect for RetryEffect {
    type Output = ();
    type Error = RetryError;
}

// Free monad or effect interpreter pattern
enum TaskProgram<A> {
    Pure(A),
    Effect {
        effect: Box<dyn Effect>,
        continuation: Box<dyn Fn(Effect::Output) -> TaskProgram<A>>,
    },
}

// Decider returns a "program" (effect description)
fn decide(...) -> TaskProgram<TaskStatus> {
    TaskProgram::Effect {
        effect: Box::new(RetryEffect { delay: 2s, ... }),
        continuation: Box::new(|_| TaskProgram::Pure(TaskStatus::Retrying)),
    }
}
```

**利点:**
- Effect composition が自然
- Handler の合成が容易
- 型安全性が最大限

**欠点:**
- 複雑性が増す
- Rust の型システムでは実装が難しい（GAT, dyn Trait の制限）

### Option C: Async Trait を Effect として活用

Rust の `async/await` 自体が一種の algebraic effect なので：

```rust
#[async_trait]
trait AsyncDecider {
    // 各 effect を trait method として提供
    async fn retry(&self, delay: Duration) -> Result<(), Error>;
    async fn decompose(&self, tasks: Vec<TaskSpec>) -> Result<(), Error>;
    async fn request_input(&self, prompt: String) -> Result<String, Error>;

    // Decider が "effect を実行する" ように見える
    async fn decide(&self, task: &TaskRecord, outcome: &Outcome) -> TaskStatus {
        match outcome.kind {
            OutcomeKind::Failure => {
                self.retry(Duration::from_secs(2)).await?;
                TaskStatus::Retrying
            }
            OutcomeKind::Blocked => {
                let input = self.request_input("Need more info").await?;
                // ... process input
                TaskStatus::Running
            }
            _ => TaskStatus::Completed,
        }
    }
}
```

**利点:**
- Rust の async ecosystem と自然に統合
- Effect の実装を外部で提供可能（dependency injection）

**欠点:**
- Effect trace の記録が自動化しにくい
- 純粋性が失われる（async は副作用を含む）

## 結論

### 現状の評価

Weaver の設計は、意図的かどうかはともかく、algebraic effects の核心的なパターンを体現している：

✅ **すでに実現されている:**
- 純粋な計算（Decider）と副作用（Handler/Worker）の分離
- 代数的データ型（Decision enum）による effect 表現
- Pattern matching による網羅的な handling
- Effect trace（AttemptRecord/DecisionRecord）の記録

🔄 **さらに活かせる可能性:**
- Effect composition（複数の handler を chain）
- Effect polymorphism（generic effect handling）
- Resumption/continuation の明示化
- Type-level effect tracking

### 実装方針

**現時点（2026-01-01）:**
- より明示的な algebraic effects system は実装しない
- 既存の `Decision` enum + `Decider` trait パターンを維持
- 必要に応じて `Decision` の variant を追加（Option A）

**将来的な可能性:**
- Phase 4-2 以降で `Decision` を拡張（Decompose, AddDependency 等）
- Effect composition が必要になったら再検討
- Rust の型システムの進化（GAT の安定化等）を見て Option B を検討

## 参考文献

- Pretnar, M. (2015). "An Introduction to Algebraic Effects and Handlers"
- Leijen, D. (2017). "Type Directed Compilation of Row-Typed Algebraic Effects" (Koka)
- Kammar et al. (2013). "Handlers in Action"
- Bauer, A., & Pretnar, M. (2015). "Programming with Algebraic Effects and Handlers"

## 関連ドキュメント

- Requirements: `dev/docs/requirements/2025_12_27_weaver_requirements.md`
- Decision model: `crates/weaver-core/src/domain/decision.rs`
- Attempt/Decision records: `crates/weaver-core/src/domain/attempt.rs`
- Functional programming requirements: requirements:236-244

## メタデータ

- **検討日**: 2026-01-01
- **検討者**: Learning session
- **ステータス**: 探索的考察（実装予定なし）
- **次のアクション**: なし（記録のみ）
