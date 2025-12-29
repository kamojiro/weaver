# ADR-0001: TaskId のみを保持する3つのデータ構造分離

- 日付: 2025-12-28
- ステータス: 承認
- レイヤ: core
- 種別: データストア / ドメインモデル
- 関連コンポーネント: `weaver-core::queue::memory::InMemoryQueueState`

---

## 1. 背景 / コンテキスト

InMemoryQueue の内部データ構造をどう設計するかを決定する必要がある。

- **目的・解決したいこと:**
  - タスクの状態管理と実行キューを効率的に実装する
  - Rust の所有権システムと調和する設計
  - 状態の不整合を防ぎ、Single Source of Truth を確保

- **前提・制約:**
  - v1 はシングルプロセス・インメモリ実装
  - 複数の Worker が並行してタスクを取得・実行する
  - リトライ時は遅延（バックオフ）後に再実行
  - Rust の所有権ルールを守る必要がある

- **この ADR がカバーする範囲:**
  - InMemoryQueueState の内部データ構造のみ
  - 状態遷移ロジックは ADR-0002 で扱う
  - 非同期パターンは ADR-0003, ADR-0004 で扱う

---

## 2. 決定

**3つのデータ構造に分離し、キュー構造には TaskId のみを保持する。**

```rust
struct InMemoryQueueState {
    records: HashMap<TaskId, TaskRecord>,      // すべてのタスク（実体）
    ready: VecDeque<TaskId>,                   // 実行可能なタスク（ID のみ）
    scheduled: BinaryHeap<ScheduledTask>,      // リトライ待機中（ID + 時刻）
}
```

- `records` が TaskRecord の唯一の所有者（Single Source of Truth）
- `ready` と `scheduled` はインデックス（ビュー）として機能
- TaskId は軽量（u64）なので、コピーコストが低い

---

## 3. 選択肢と評価

### 採用案（本 ADR の決定）

**3つのデータ構造 + TaskId のみ保持**

- **概要:**
  - `records: HashMap<TaskId, TaskRecord>` で実体を一元管理
  - `ready: VecDeque<TaskId>` で実行可能なタスク ID を FIFO 管理
  - `scheduled: BinaryHeap<ScheduledTask>` でリトライ待機中のタスクを時刻順管理

- **メリット:**
  - ✅ 所有権が明確（records が唯一の所有者）
  - ✅ 状態の不整合が起きにくい（Single Source of Truth）
  - ✅ キューの操作が軽量（ID の移動だけ、8バイト）
  - ✅ HashMap の O(1) lookup で実体にアクセス
  - ✅ ロック競合が少ない（データ構造が独立）

- **デメリット / リスク:**
  - ⚠️ ID → TaskRecord の lookup が必要（ただし O(1)）
  - ⚠️ 3つの構造の同期を保つ必要がある（ロジックで管理）

### 代替案 A: すべて HashMap（キューなし）

**概要:**
```rust
struct AlternativeA {
    records: HashMap<TaskId, TaskRecord>,
    // ビューなし、毎回イテレーションで探す
}
```

**採用しなかった理由:**
- ❌ `lease()` が O(n) になる（全タスクをスキャン）
- ❌ 次のリトライ時刻を見つけるのが遅い
- ❌ 実行可能なタスクを効率的に取得できない

### 代替案 B: TaskRecord を Arc で共有

**概要:**
```rust
struct AlternativeB {
    records: HashMap<TaskId, Arc<Mutex<TaskRecord>>>,
    ready: VecDeque<Arc<Mutex<TaskRecord>>>,  // 同じ Arc を共有
    scheduled: BinaryHeap<Arc<Mutex<TaskRecord>>>,
}
```

**採用しなかった理由:**
- ❌ 状態遷移のたびに Mutex のロック競合
- ❌ メモリオーバーヘッド（Arc + Mutex のコスト）
- ❌ デッドロックのリスク増加
- ❌ 複数のキューで同じ TaskRecord を共有すると、状態管理が複雑化

### 代替案 C: TaskRecord を clone してキューに格納

**概要:**
```rust
struct AlternativeC {
    records: HashMap<TaskId, TaskRecord>,
    ready: VecDeque<TaskRecord>,  // clone したものを格納
}
```

**採用しなかった理由:**
- ❌ メモリ使用量が増加（TaskRecord は大きい構造体）
- ❌ 状態の同期が必要（records と ready で二重管理）
- ❌ どちらが正しい状態か不明確になる

---

## 4. 根拠（評価軸と判断）

### ビジョンとの整合

- **Single Source of Truth**: 要件ドキュメントで求められる「説明可能性」を実現するため、状態の一元管理が必要
- **関数型アプローチ**: データ（records）とビュー（ready/scheduled）の分離は、関数型的な設計に合致

### 非機能要件

- **パフォーマンス**: TaskId（8バイト）の移動は高速。HashMap の O(1) lookup も効率的
- **並行性**: データ構造が独立しているため、将来的に細粒度ロックも可能
- **メモリ効率**: TaskRecord の clone を避け、メモリ使用量を最小化

### Rust の所有権との整合

- **所有権の明確化**: records が唯一の所有者、他は ID のみ
- **借用チェッカーとの調和**: `get_mut()` で可変借用を取得し、状態遷移を実行
- **コンパイラの保証**: 参照の寿命問題を根本的に回避

---

## 5. 影響範囲

### コード / ディレクトリ構成

- `crates/weaver-core/src/queue/memory.rs`:
  - `InMemoryQueueState` 構造体の定義
  - `promote_scheduled_tasks()` で scheduled → ready の昇格
  - `lease()` で ready から TaskId を取得し、records から実体を lookup

### 既存・将来のコンポーネント

- **Queue trait**: TaskId ベースの設計により、将来的な永続化実装（DB など）でも同様のパターンを適用可能
- **Worker**: TaskLease を通じて TaskEnvelope を受け取るため、内部構造を意識しない
- **将来の拡張**: 優先度キュー、依存関係グラフなども同様に ID ベースで実装可能

### 運用プロセス

- **デバッグ**: `records` を見れば全タスクの状態が分かる
- **監視**: `counts_by_state()` で records を1回イテレーションするだけ

---

## 6. ロールアウト / 移行方針

### 実装状況

- **ステータス**: 実装済み（2025-12-28）
- **コミット**: `53f3b96` "wip"

### 実装済みの要素

1. ✅ `InMemoryQueueState` の3つのデータ構造定義
2. ✅ `promote_scheduled_tasks()` による自動昇格
3. ✅ `lease()` での ID ベース取得
4. ✅ `enqueue()` での records への追加と ready への ID 追加
5. ✅ `counts_by_state()` での observability

---

## 7. オープンな論点 / フォローアップ

### 将来的な検討事項

- **永続化実装**: DB ベースの Queue 実装時も同様のパターンを適用するか？
  - 候補: `task_id` をインデックスとして使用し、状態は別テーブルで管理
- **細粒度ロック**: 現在は `InMemoryQueueState` 全体をロック。将来的に records と ready/scheduled を分離してロックするか？
- **優先度キュー**: 優先度を持つタスクの場合、ready を BinaryHeap に変更するか？

### 関連する設計判断

- ADR-0002 で扱う「状態遷移の集約」と相互補完的
- ADR-0003 で扱う「ロック外 notify」がこの設計の効果を最大化

---

## 8. 関連 ADR

- **ADR-0002**: TaskRecord への状態遷移の集約（本設計の Single Source of Truth を補完）
- **ADR-0003**: ロック外での notify による非同期安全性（本設計によるロック最小化を活用）
- **ADR-0004**: tokio::select! による複数イベント待機（scheduled の時刻管理と連携）
