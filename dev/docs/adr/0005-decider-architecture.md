# ADR-0005: Decider アーキテクチャ

## ステータス

**承認済み (Accepted)** - 2026-01-02

- **提案日**: 2025-12-30
- **実装開始**: 2026-01-01
- **実装完了**: 2026-01-02
- **承認日**: 2026-01-02

Phase 4-1 の実装が完了し、Handler → Outcome → Decider → Decision フローが正常に動作することを確認しました。

## コンテキスト

Phase 4（Task 分解）の設計において、「次に何をすべきか」を判断する仕組みが必要。
現在の実装では、TaskLease::fail() 内でリトライ判断が直接実装されているが、これでは：

1. 判断ロジック（純粋関数）と実行（副作用）が混在
2. 拡張性が低い（分解、依存追加など新しい判断を追加しにくい）
3. ユーザーがカスタム判断ロジックを追加できない

要件では「Decider は純粋関数として実装」と明記されており、関数型アプローチを活かす設計が求められている。

## 決定事項

### 1. Decider trait の導入

次の操作を決定する trait を定義する：

```rust
pub trait Decider {
    fn decide(&self, task: &TaskRecord, outcome: &Outcome) -> Decision;
}
```

**入力:**
- `&TaskRecord`: タスクの状態（attempts, max_attempts など）
- `&Outcome`: 最新の実行結果（SUCCESS/FAILURE/BLOCKED）

**出力:**
- `Decision`: 次に取るべきアクション（Retry/MarkDead/Decompose など）

### 2. Decision enum の定義

次のアクションを表現する enum：

**Phase 4-1:**
```rust
pub enum Decision {
    Retry { delay: Duration, reason: String },
    MarkDead { reason: String },
}
```

**Phase 4-2 以降:**
```rust
pub enum Decision {
    Retry { delay: Duration, reason: String },
    MarkDead { reason: String },
    Decompose { subtasks: Vec<TaskSpec> },  // 追加予定
    AddDependency { task_id: TaskId },      // 追加予定
}
```

### 3. 2層の Decider アーキテクチャ

**weaver-core が提供する Decider:**
- `DefaultDecider`: attempt ベース/budget ベースの判断
- リトライ回数、バックオフ、deadline などの汎用的な判断
- すべての weaver-core ユーザーが利用可能

**ユーザーが実装するカスタム Decider:**
- AI Agent による高度な判断（タスク分解、代替戦略など）
- ドメイン固有のロジック
- **weaver-core のスコープ外**（ユーザーが trait を実装）

### 4. 責務分担

| コンポーネント | 責務 | 純粋関数 | 副作用 |
|--------------|------|---------|--------|
| **Handler** | タスク実行、Outcome 返却 | ❌ | ✅ 実行 |
| **Decider** | 次のアクション決定 | ✅ | ❌ |
| **Worker/TaskLease** | Decision の実行 | ❌ | ✅ 状態更新 |

**フロー:**
```
Handler::execute() → Outcome
   ↓
Decider::decide(task, outcome) → Decision
   ↓
Worker/TaskLease が Decision を実行（副作用）
```

### 5. Handler は Outcome を返す

Handler trait を変更し、Outcome を返すようにする：

**変更前（Phase 1-3）:**
```rust
trait TaskHandler {
    async fn execute(&self, envelope: &TaskEnvelope) -> Result<(), WeaverError>;
}
```

**変更後（Phase 4）:**
```rust
trait TaskHandler {
    async fn execute(&self, envelope: &TaskEnvelope) -> Result<Outcome, WeaverError>;
}
```

**理由:**
- Handler が SUCCESS/FAILURE/BLOCKED を判断し、Outcome として返す
- Decider は Outcome を受け取って次のアクションを決定
- 要件の「観測/結果 → 次の操作」に対応

### 6. Success 時は Decider を呼ばない

- Outcome::Success の場合、Decider を呼ばずに ack() して終了
- Decider は Outcome::Failure/Blocked の場合のみ呼ぶ
- 理由: 成功時に判断する必要がない（Handler が既に判断済み）

**成功結果の検証（Future work）:**
- 「Outcome::Success だが本当に成功か？」の検証は将来の拡張
- Phase 4-1 では Handler の判断を信じる

## 代替案

### 代替案 A: TaskLease::complete(outcome) を追加

**概要:**
- Handler は変更せず、TaskLease に complete メソッドを追加
- 既存の ack/fail は維持

**却下理由:**
- API が複雑化（ack/fail/complete の3つ）
- Worker の実装が複雑（outcome に応じて ack/fail/complete を選ぶ）
- Handler の責務が不明確

### 代替案 B: Decider が Outcome も決定

**概要:**
- Handler は生のデータ（LlmResponse など）を返す
- Decider が Outcome を決定し、さらに Decision も決定

**却下理由:**
- Decider の責務が過剰（Outcome の解釈 + 次のアクション決定）
- Handler の責務が不明確
- 要件の「観測/結果 → 次の操作」から逸脱

## 結果

### メリット

1. **純粋関数と副作用の分離**
   - Decider は純粋関数（テストしやすい、推論しやすい）
   - 副作用は Handler と Worker/TaskLease に集約

2. **拡張性**
   - Decision に新しいバリアント追加が容易（Decompose, AddDependency など）
   - ユーザーがカスタム Decider を実装可能

3. **要件との整合性**
   - 「Decider は純粋関数」という要件を満たす
   - 「観測/結果 → 次の操作」のフローを明確に実装

4. **テスタビリティ**
   - Decider のロジックを単体テストしやすい（入力 → 出力の関数）
   - Handler の実行と判断ロジックを独立してテスト可能

### デメリット

1. **既存コードの変更**
   - TaskHandler trait のシグネチャ変更
   - TaskLease の変更（complete メソッド追加）
   - Worker の変更（新しいフロー）

2. **複雑性の増加**
   - コンポーネント数が増える（Handler, Decider, Worker/TaskLease）
   - フローが長くなる（Handler → Outcome → Decider → Decision → 実行）

### トレードオフ

v1 では正しい設計を追求し、移行コストを受け入れる。
設計の不確実性は実装しながら学ぶアプローチで対処。

## 実装計画

### Phase 4-1: 最小限の Decider（retry のみ）✅ 完了

1. ✅ Decision enum の定義（Retry/MarkDead）
2. ✅ Decider trait の定義（Send + Sync 制約追加）
3. ✅ DefaultDecider の実装（RetryPolicy ロジックを移行）
4. ✅ TaskHandler trait の変更（Outcome を返す）
5. ✅ TaskLease の変更（complete メソッド、get_task_record メソッド）
6. ✅ Worker の変更（新しいフロー、Decider 統合）
7. ✅ テスト（complete() の単体テスト、CLI での動作確認）

**実装ファイル:**
- `crates/weaver-core/src/domain/decision.rs` - Decision, Decider, DefaultDecider
- `crates/weaver-core/src/domain/outcome.rs` - Outcome, OutcomeKind, Artifact
- `crates/weaver-core/src/runtime.rs` - TaskHandler trait の変更
- `crates/weaver-core/src/worker.rs` - Worker loop フロー実装
- `crates/weaver-core/src/queue/mod.rs` - TaskLease trait の拡張
- `crates/weaver-core/src/queue/memory.rs` - complete(), get_task_record() 実装
- `crates/weaver-cli/src/main.rs` - HelloHandler の更新、DefaultDecider 統合

**テスト:**
- `test_complete_with_retry_decision` - Retry パスのテスト
- `test_complete_with_mark_dead_decision` - MarkDead パスのテスト
- `test_complete_creates_both_records` - AttemptRecord + DecisionRecord の作成確認
- CLI 実行テスト - end-to-end フロー動作確認

### Phase 4-2: 分解機能の追加

1. Decision::Decompose の追加
2. 親子関係の実装（TaskRecord に parent_id/children 追加）
3. 分解ロジックの実装
4. テスト

### Future Work

1. **Chain of Responsibility パターン**
   - 複数の Decider を組み合わせる仕組み
   - DeciderChain の実装

2. **成功結果の検証**
   - Outcome::Success の妥当性チェック
   - Handler の判断を Decider が補正する仕組み

3. **Budget ベースの判断**
   - deadline 超過の判断
   - cost 上限到達の判断

4. **カスタム Decider の例**
   - LlmAgentDecider の実装例
   - ドキュメント化

## 参考資料

- 要件: `dev/docs/requirements/2025_12_27_weaver_requirements.md` (セクション 6.2, 11)
- 学習記録: `dev/learning/2025_12_30.md` (Phase 4 設計議論)
- ADR-0001: TaskId のみを保持する3つのデータ構造分離
- ADR-0002: TaskRecord への状態遷移の集約
- ADR-0003: ロック外での notify による非同期安全性

## メモ

### 実装履歴

**提案（2025-12-30）:**
この ADR は Phase 4-1 の実装途中で作成されました。実装を通じて設計の妥当性を検証することを計画しました。

**実装完了（2026-01-02）:**
Phase 4-1 の実装を完了し、以下の点について検証しました：

1. **Decider の入力として TaskRecord + Outcome で十分か**
   - ✅ 十分であることを確認。DefaultDecider は TaskRecord.attempts と max_attempts のみを使用し、Outcome の kind は将来の拡張用に保持。

2. **DefaultDecider の実装における RetryPolicy の扱い**
   - ✅ DefaultDecider が RetryPolicy を保持する設計で実装。
   - ✅ `DefaultDecider::default_v1()` convenience constructor を提供。

3. **Worker での Decision 実行の具体的な実装方法**
   - ✅ `TaskLease::complete(outcome, decision)` メソッドで Decision を実行。
   - ✅ ADR-0003（lock-outside-notify）に準拠した実装。
   - ✅ SUCCESS 時は Decider をバイパスして ack() を直接呼ぶ最適化を実装。

### 実装で得られた知見

1. **インフラエラーとビジネスエラーの区別**
   - `Err(WeaverError)`: インフラエラー → Worker が `Outcome::failure()` に変換
   - `Ok(Outcome::failure())`: ビジネスエラー → Decider が判断
   - この区別により、Handler が実行結果を観測・報告する責務を果たせる。

2. **Send + Sync 制約の必要性**
   - Decider trait に Send + Sync を追加する必要があった（tokio::spawn で Arc<dyn Decider> を渡すため）。

3. **テスタビリティの向上**
   - Decider が純粋関数であるため、単体テストが容易。
   - complete() メソッドのテストで AttemptRecord と DecisionRecord の両方が作成されることを確認。

### 次のステップ

Phase 4-2 では、Decision::Decompose を追加し、タスク分解機能を実装する予定です。
