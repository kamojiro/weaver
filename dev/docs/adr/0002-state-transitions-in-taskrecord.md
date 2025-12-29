# ADR-0002: 状態遷移ロジックを TaskRecord に集約

- 日付: 2025-12-28
- ステータス: 承認
- レイヤ: core
- 種別: ドメインモデル
- 関連コンポーネント: `weaver-core::queue::record::TaskRecord`

---

## 1. 背景 / コンテキスト

タスクの状態遷移ロジックをどこに配置するかを決定する必要がある。

- **目的・解決したいこと:**
  - タスク状態遷移時の不変条件（invariants）を保護する
  - 状態遷移を明示的にし、コードの可読性を向上させる
  - 将来の拡張（イベント発行、メトリクス送信など）に備える

- **前提・制約:**
  - TaskState は enum（Queued/Running/Succeeded/RetryScheduled/Dead）
  - 状態遷移時に複数フィールド（state, attempts, updated_at など）を同時更新する必要がある
  - カプセル化とドメインロジックの保護が重要

- **この ADR がカバーする範囲:**
  - TaskRecord 内の状態遷移メソッドの設計
  - Queue や Worker から状態遷移をどう呼び出すかの方針

---

## 2. 決定

**すべての状態遷移を TaskRecord の専用メソッドとして実装する。**

各状態遷移に対応するメソッドを提供：
- `start_attempt()`: Queued → Running
- `mark_succeeded()`: Running → Succeeded
- `mark_dead(error)`: Running → Dead
- `schedule_retry(next_run_at, error)`: Running → RetryScheduled
- `requeue()`: RetryScheduled → Queued

各メソッドは関連フィールド（attempts, updated_at, last_error など）を原子的に更新する。

---

## 3. 選択肢と評価

### 採用案（本 ADR の決定）

**TaskRecord に状態遷移メソッドを集約**

- **メリット:**
  - 不変条件を保護（複数フィールドの一貫性を保証）
  - 状態遷移が明示的（メソッド名で意図が明確）
  - カプセル化（外部から直接フィールドを変更させない）
  - 拡張性（メソッド内に追加ロジックを挿入しやすい）

- **デメリット / リスク:**
  - TaskRecord が「データ + ロジック」を持つ（純粋なデータ構造ではない）
  - メソッド数が増える可能性

### 代替案 A: フィールドを public にして外部で変更

- **概要:** TaskRecord のフィールドを pub にし、Queue 側で直接変更
- **採用しなかった理由:**
  - 不変条件を守れない（updated_at の更新忘れ、state と他フィールドの不整合）
  - 状態遷移ロジックが Queue に分散し、保守性が低下

### 代替案 B: 状態遷移ロジックを Queue に配置

- **概要:** TaskRecord はデータのみ、Queue が状態遷移ロジックを持つ
- **採用しなかった理由:**
  - ロジックが分散（TaskRecord と Queue の両方を見る必要）
  - TaskRecord が「貧血ドメインモデル」になる
  - Queue が肥大化する

---

## 4. 根拠（評価軸と判断）

- **ビジョンとの整合:**
  - 関数型アプローチ：状態遷移を明示的な関数として表現
  - 説明可能性：各遷移がメソッドとして記録され、追跡可能

- **非機能要件:**
  - 保守性：状態遷移ロジックが一箇所に集約され、理解しやすい
  - 拡張性：将来のイベント発行やメトリクス送信を各メソッドに追加可能

- **Rust のベストプラクティス:**
  - カプセル化：フィールドを private にし、メソッド経由でのみ変更
  - 型安全性：不正な状態遷移をコンパイル時に防げる（将来的に型状態パターンも可能）

---

## 5. 影響範囲

- **コード:**
  - `crates/weaver-core/src/queue/record.rs`: 5つの状態遷移メソッド
  - `crates/weaver-core/src/queue/memory.rs`: TaskRecord のメソッドを呼び出す
  - `crates/weaver-core/src/worker.rs`: ack/fail 時に間接的に状態遷移

- **将来のコンポーネント:**
  - Decision 記録（ADR で今後扱う）: 各状態遷移時に Decision を記録可能
  - イベント発行：メソッド内でイベントを emit する拡張が容易

---

## 6. ロールアウト / 移行方針

- **実装状況:** 実装済み（2025-12-28、コミット `53f3b96`）
- **実装済みの要素:**
  - 5つの状態遷移メソッド
  - 各メソッドでの updated_at 自動更新
  - 関連フィールド（attempts, last_error, next_run_at）の一貫性保証

---

## 7. オープンな論点 / フォローアップ

- **型状態パターンの導入:** 将来的に `TaskRecord<Queued>`, `TaskRecord<Running>` のような型レベルでの状態管理を検討する価値がある
- **Decision 記録との統合:** Phase 3 で Attempt/Decision 記録を実装する際、各状態遷移メソッド内で Decision を記録する
- **イベント駆動アーキテクチャ:** 状態遷移時にイベントを発行し、外部システムと連携する可能性

---

## 8. 関連 ADR

- **ADR-0001**: TaskId のみを保持する3つのデータ構造分離（本設計の Single Source of Truth を実現）
- **ADR-0003**: ロック外での notify による非同期安全性（状態遷移メソッドをロック内で安全に呼び出す）
