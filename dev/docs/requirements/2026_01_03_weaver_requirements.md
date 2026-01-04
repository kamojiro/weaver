# Weaver Requirements (v2 / v3)
**ファイル名:** `2026_01_03_weaver_requirements.md`  
**作成日:** 2026-01-03（ファイル名は指定に従う）  
**目的:** これまでの議論を **v2 実装要件**として確定し、**v3 バックログ**を整理する。

---

## 編集プロセス（指示された反復）
このドキュメントは以下の手順を **4回以上**繰り返して精査している。

- **反復1:** 章立て（大項目・中項目・小項目）を作成
- **反復2:** 議論内容を各項目に肉付け
- **反復3:** 議論外だが要件に効く提案を追記（非目標も明確化）
- **反復4:** 不足/矛盾/実装不能点を見直し、v2/v3の切り分けを再調整
- **反復5:** コード構造（module tree）と ports（trait署名）を確定し、2週間計画に落とし込み

---

## 0. 合意済みの大方針（サマリ）

### 0.1 ソースオブトゥルース
- **PostgreSQL を正本（source of truth）**とする
- **Redis は配送（Delivery）の派生物**。Redisの状態は PG から再構築可能であること

### 0.2 実行保証
- **at-least-once** を前提（重複実行は起こりうる）
- 重複実行の吸収は基本 **Handler 側**（必要なら下流ストレージの UNIQUE 制約や idempotency key）

### 0.3 Payload / 巨大コンテキスト
- payload は原則 **artifact_ref（Blob参照）**に寄せる
- **TTL（expires_at）** をサポートし、GCで掃除可能にする
- PG/Redis に巨大データを直埋めしない（最大LLM文脈長級でも設計不変）

### 0.4 型付き Task API（二層構造）
- 内部永続表現は `task_type: String` を維持
- 利用者向けには **型付きTask API** を提供（Task trait / Handler<T> / TypedRegistry / enqueue_typed）

### 0.5 DispatchStrategy
- v2 では **概念（trait）**を入れ、デフォルトは **Direct（task_type 1:1）**
- 将来（v3）で RuleDispatch / AgentDispatch を追加できる形を守る

### 0.6 Decode失敗は復旧可能
- Decode/Deserialize 失敗は **infra Err にしない**
- Outcome/Decision に落として処理継続し、**repair/regenerate**で復旧可能にする
- 無限ループ防止として **max_repairs** を導入

### 0.7 依存関係 = 順序
- job内の「ふんわりした順序」は **依存関係（DAG）**で表現（明示的順序キュー不要）
- ready 判定は DAG の満了（remaining_deps==0）で行う

### 0.8 Docker Compose（全部入り）
- v2 で「全部入り（PG/Redis/Worker/Publisher/Blob/任意の観測）」が `docker compose up` で起動できる

---

## 1. v2 の目標・非目標・制約

### 1.1 v2 の目標（MUST）
1. **PG正本 + outbox** により状態・履歴・依存・配送指示が確実に残る
2. **Redis配送**（または InMemory配送）によりワーカーが task_id を受け取って実行できる
3. **Typed Task API**で task_type typo を排除し、**起動時検証**で登録漏れを検知できる
4. **Lease + visibility timeout** によりワーカー死活に強く、at-least-once を成立させる
5. **Decode失敗の復旧フロー**（repair task + hint）と **無限ループ防止**
6. **Artifact/TTL/GC** により巨大コンテキストを扱える
7. **docker compose** により統合動作が再現できる（ローカルで検証可能）

### 1.2 v2 の非目標（明示）
- データ保護（暗号化・マスキング・アクセス制御）は **扱わない**
- バックプレッシャ/レート制御は **扱わない**（先で検討）
- 強制 cancel（kill/強制中断）は **扱わない**（best-effort のみ）
- “全タスク登録済み”を型レベルで保証（compile-time完全性）は **扱わない**（起動時検証で十分）
- イベントソーシング（events正本 + projection）は **採用しない**（現状態 + 履歴）

### 1.3 制約（2週間実装）
- Claude Code Pro を前提に **約2週間**で v2 の核を成立させる
- そのため v2 は「最小の正しさ」を優先し、拡張は v3 に送る

---

## 2. 用語
- **Task**: 実行単位。task_type + payload参照 + budget/期限/依存など
- **Job**: 複数Taskのまとまり（予算/期限/依存のスコープ）
- **Handler**: Taskを実行し Outcome を返すビジネスロジック
- **Outcome**: 実行結果（success/failure/decompose/blocked/cancelled など）
- **Decision**: Outcome と予算/期限等から次アクションを決めた結果
- **Lease**: 実行権（visibility timeout で回収されうる）
- **TaskStore**: PG正本（状態・履歴・依存・outbox生成）
- **DeliveryQueue**: 配送キュー（InMemory/Redis で差し替え）。task_idのみ流す
- **Outbox**: PGトランザクション内で「配送すべきイベント」を積むテーブル
- **ArtifactStore**: Blob（S3/MinIO/Local）管理

---

## 3. weaver-core ソースアーキテクチャ（v2要件）

### 3.1 ねらい：複雑化を防ぐ「責務の割り当て」
Redis/PG/Blob を入れると、v1の“Queue中心”構造は責務が混ざりやすい。  
v2では **正本（PG）と配送（Redis/InMemory）を分割**し、整合性は PG に閉じる。

### 3.2 推奨 module tree（v2）
> adapters は別クレート推奨。v2で速度優先なら feature 同梱も可。ただし ports の背後に隠す。

```
weaver-core/
  src/
    lib.rs

    domain/
      mod.rs
      ids.rs                  # ULID newtypes
      task_type.rs            # 命名規約支援（バリデータ等）
      envelope.rs             # TaskEnvelope（task_type, artifact_ref, schema_version, meta）
      budget.rs
      outcome.rs
      decision.rs
      state.rs                # TaskState/JobState + waiting_reason
      errors.rs               # ErrorKind 等（運用分類）
      events.rs               # DomainEvent（最小）

    ports/
      mod.rs
      task_store.rs           # TaskStore trait（正本）
      delivery_queue.rs       # DeliveryQueue trait（配送）
      artifact_store.rs       # ArtifactStore trait（Blob）
      decider.rs              # Decider trait（将来 chain）
      dispatch.rs             # DispatchStrategy trait（Directはapp側でも可）
      repair_hint.rs          # RepairHintGenerator trait
      clock.rs
      id_generator.rs
      event_sink.rs           # 任意（v2最小: noop）

    app/
      mod.rs
      builder.rs              # AppBuilder（expect_tasks, wiring）
      runtime.rs              # Typed APIの表面（enqueue_typedなど）
      worker_loop.rs          # WorkerLoop（pop->claim->handle->decide->complete）
      publisher_loop.rs       # OutboxPublisherLoop（outbox->push->ack）
      reaper_loop.rs          # LeaseReaperLoop（期限切れ回収）
      gc_loop.rs              # Artifact GC（expires_at）
      status.rs               # status query（詰まり理由を説明）

    typed/
      mod.rs
      task.rs                 # trait Task { TYPE }
      handler.rs              # trait Handler<T: Task>
      registry.rs             # TypedRegistry -> DynHandler adapter
      codec.rs                # PayloadCodec（artifact->T decode）

    impls/
      mod.rs
      inmem_delivery.rs       # InMemoryDeliveryQueue（開発用）
      # TaskStore/ArtifactStore の inmem 実装はテスト用にのみ置く（必要なら）
```

#### 推奨：アダプタは別クレート
```
weaver-pg/       # TaskStore(Postgres) + migrations
weaver-redis/    # DeliveryQueue(Redis)
weaver-blob/     # ArtifactStore(MinIO/S3/Local)
weaver-cli/      # サンプルアプリ（Typed Task + Handler群 + compose起動）
```

### 3.3 シンプル化のルール（v2）
- Redis には **task_id だけ**を流す（envelope/payload/状態は置かない）
- Lease の正本は PG（Redis を信じない）
- 状態モデルは「現在状態 + 履歴」（event sourcing はしない）
- 巨大データは最初から Blob に逃がす（参照のみ）

---

### 3.4 不変条件（MUST）
v2 の実装において、設計の簡素化と整合性を守るために以下を **不変条件**として固定する。

- **Redis は配送（Delivery）に限定**し、保持してよいのは **task_id（+軽量メタ）**のみ  
  - Redis に **状態・payload・envelope** を保存しない
- **Lease の正本は PostgreSQL** とし、Redis の `pop` は **実行候補の通知**に過ぎない  
  - 実行権の確定は `TaskStore.claim()` の成功でのみ決まる
- Task が `ready` になった場合、同一トランザクションで **必ず `outbox_events` に `dispatch_task` を append** する  
  - `ready` と配送が断絶して「配送し忘れ」が起きる設計を禁止する
- v2 では依存関係は **作成時に確定**し、実行開始後に依存を追加しない（または強い制約下のみ許可）  
  - `remaining_deps` による ready 判定の整合を単純化する
- payload は原則 **artifact_ref**。巨大データを PG/Redis に直埋めしない  
  - サイズ上限を設け、超過時は artifact を強制する
- 状態遷移（claim/complete/reap）と outbox の生成は **TaskStore のトランザクション境界**に閉じる  
  - app 層は状態を“直接”いじらず、ports の呼び出しでのみ進める


## 4. TaskType 命名規約（v2要件）
### 4.1 形式
- `{namespace}.{domain}.{action}.v{major}`
  - 例: `acme.billing.charge.v1`

### 4.2 ルール
- major だけを task_type に含める（互換破壊時に上げる）
- minor/patch は payload の `schema_version` で吸収
- 文字集合は `[a-z0-9_.]` 推奨（運用・SQL・監視で扱いやすい）

---

## 5. 型付き Task API（二層）（v2要件）

### 5.1 目的
- Task投入側と Handler登録側を **型で結び**、task_type typo を排除
- 保存/通信表現としての `task_type: String` を維持し、将来の動的ルーティングも可能にする

### 5.2 MUST 機能
- `trait Task { const TYPE: &'static str; }`
- `trait Handler<T: Task>`
- `TypedRegistry::register::<T>(handler)` が型で対応付けされる
- `enqueue_typed<T: Task>(t)` を提供し、内部的に `TaskEnvelope{ task_type: T::TYPE, ... }` を生成
- internal は object-safe `DynHandler` に落とす（deserialize + handle）

### 5.3 起動時検証（登録漏れ）
- v2 は起動時検証
  - `AppBuilder::expect_tasks([...])`
  - build 時に「期待集合 ⊆ 登録済み集合」をチェックし fail-fast

---

## 6. DispatchStrategy（v2要件）
- `DispatchStrategy` trait を導入
- v2 のデフォルトは `DirectDispatch`（task_type 1:1）
- v3 で Rule/Agent を追加可能な形にする（app→ports 経由）

---

## 7. 実行保証：Lease / at-least-once / 重複（v2要件）

### 7.1 Lease / visibility timeout
- `claim(task_id)` は PG で lease を発行し `lease_expires_at` を持つ
- worker 死亡などで lease が期限切れになったら回収し、再配送可能
- at-least-once を成立させる

### 7.2 重複吸収（idempotency）
- 既定: Handler が重複に耐える（下流DBの UNIQUE 制約など）
- 任意拡張: idempotency key（例: task_id）で “二重適用” を無害化

---

## 8. 依存関係（DAG）と “ふんわり順序”（v2要件）
- 依存関係は DAG として表現
- ready 判定は `remaining_deps==0` を基本にする
- 依存追加/依存解放は PG トランザクション内で `remaining_deps` を更新する（joinで毎回判定しない）

---

## 9. Cancel / Deadline / Budget（v2要件）

### 9.1 Cancel（best-effort）
- cancel は best-effort
  - 未開始は cancelled へ
  - 実行中は cooperative（handler がフラグ確認できる余地）
- 強制 kill は v3 以降

### 9.2 Deadline（自然な挙動）
- deadline 超過時は「これ以上進めない」
  - 未開始: cancelled（deadline_exceeded など理由を残す）
  - 実行中: best-effort cancel

### 9.3 Budget（job/task/attempt で設定可能）
- job/task/attempt それぞれ設定可能
- 継承ルールで爆発を防ぐ（Job → Task → Attempt）
- v2最小の budget 種:
  - max_attempts
  - max_repairs（decode復旧回数）

---

## 10. Payload / Artifact / TTL（v2要件）

### 10.1 原則
- payload は artifact_ref 中心（`payload_artifact_id`）
- `schema_version` は必須

### 10.2 ArtifactStore と TTL
- artifact は `expires_at`（TTL）を持てる
- GC ループで期限切れ artifact を削除（best-effortでOK）
- store は ports の背後で差し替える（MinIO/S3/Local）

---

## 11. Decode失敗の復旧（Repair/Regenerate）と無限ループ防止（v2要件）

### 11.1 方針
- Decode失敗は infra Err ではなく Outcome/Decision で扱う（処理継続）
- repair/regenerate によって payload を作り直し、再実行できる
- 無限ループ防止: max_repairs

### 11.2 最小設計（推奨）
- 内部タスク: `weaver.internal.repair_payload.v1`
- `RepairHintGenerator` port: decode失敗の情報から “再生成のヒント” を作る
- repair成功:
  - 新しい artifact を作成し `payload_artifact_id` と `schema_version` 更新
  - 元タスクを再評価して ready 化（必要なら outbox dispatch）
- repair失敗/上限超過:
  - blocked（または failed）へ

---

## 12. PostgreSQL 最小スキーマ（v2）

> **MVPとして固定する最小セット:** `jobs / tasks / task_dependencies / attempts / decisions / outbox_events / artifacts`

### 12.1 jobs（推奨）
- PK: (namespace, job_id)
- columns: status, created_at, updated_at, deadline_at, budget_json, meta

### 12.2 tasks（正本）
- PK: (namespace, task_id)
- columns（最小）:
  - job_id, parent_task_id, task_type
  - payload_artifact_id, schema_version
  - status, waiting_reason
  - ready_at, remaining_deps
  - attempt_count, max_attempts
  - repair_count, max_repairs
  - deadline_at, cancel_requested
  - lease_id, leased_by, lease_expires_at
  - last_error_kind, last_error_json
  - created_at, updated_at

### 12.3 task_dependencies（DAG）
- PK: (namespace, task_id, depends_on_task_id)
- index: (namespace, depends_on_task_id)

### 12.4 attempts（実行履歴）
- PK: (namespace, attempt_id)
- unique: (namespace, task_id, attempt_no)
- columns: task_id, lease_id, attempt_no, started_at, finished_at, outcome_kind, error_kind, error_json, outcome_json

### 12.5 decisions（判断履歴）
- PK: (namespace, decision_id)
- columns: task_id, attempt_id, decided_at, decision_kind, next_ready_at, child_task_ids, reason_json

### 12.6 outbox_events（配送の確実化）
- PK: (namespace, event_id)
- columns: event_type, task_id, available_at, status(pending/sent/dead), attempts, last_error, created_at, sent_at, dedupe_key
- index: (namespace, status, available_at)
- optional unique: (namespace, dedupe_key)

### 12.7 artifacts（Blob参照 + TTL）
- PK: (namespace, artifact_id)
- columns: kind, store, key, sha256, size_bytes, content_type, created_at, expires_at, deleted_at, meta
- index: (namespace, expires_at)

---

## 13. 状態遷移（Task中心）と outbox イベント種（v2）

### 13.1 TaskState（v2）
- `pending` / `ready` / `running` / `succeeded` / `failed` / `cancelled` / `blocked`

### 13.2 遷移（MUST）
- `pending -> ready`
  - 条件: remaining_deps==0 かつ now>=ready_at(またはnull) かつ !cancel_requested かつ deadline未超過
  - 同一TXで outbox `dispatch_task` を積む
- `ready -> running`（claim）
  - lease発行 + attempt開始 append
- `running -> (succeeded|failed|blocked|cancelled)`（complete）
  - attempt完了 + decision append + tasks更新
  - 依存解放: downstream remaining_deps--、0なら ready 化し outbox
- `running -> pending/ready`（lease expiry 回収）
  - reaper が期限切れを戻し、必要なら outbox

### 13.3 waiting_reason（詰まり理由）
- `waiting_reason` を保持し status で説明可能にする
  - `deps`, `retry`, `repair`, `manual`, `budget`, `deadline` など

### 13.4 outbox イベント種（v2最小）
- `dispatch_task`（task_id を DeliveryQueue に push）

---

## 14. Outbox（v2要件）
### 14.1 Outbox とは
- **DB更新（PG）と配送指示を同一トランザクションで確定する**ためのパターン
- tasks の重要状態変更（例: ready 化）は、同一TXで outbox_events にイベントを append する
- publisher が outbox を読んで Redis（または InMemory）へ配送し、成功したら outbox を ack（sent）にする

### 14.2 idempotent 配送
- dedupe_key + unique などで重複配送を無害化できる設計を推奨
- ただし v2 は “最終的に届く” を優先し、完全な重複ゼロは狙わない

---

## 15. DeliveryQueue（InMemory/Redis 差し替え）（v2要件）
- `DeliveryQueue` port を定義し、実装を差し替え可能にする
- v2 で以下を提供
  - InMemoryDeliveryQueue（開発用）
  - RedisDeliveryQueue（本番想定）
- Redis では task_id のみを流す（envelope/payloadは PG/Blob）

---

## 16. Docker Compose（全部入り）（v2要件）
- `docker compose up` で以下が起動する
  - postgres（migrations込み）
  - redis
  - minio（推奨。localでも可）
  - weaver-worker（task実行）
  - outbox-publisher（同一バイナリ内の別タスクでも可）
  - lease-reaper / gc（同一バイナリ内でも可）
  - （任意）観測（otel-collector/jaeger 等）

---

## 17. 観測（v2最小）
- status で以下が分かる
  - job/task の状態
  - waiting_reason（詰まり理由）
  - 直近 attempts/decisions
- 提案（v2で入れるなら）
  - `EventSink` port（noop 実装付き）で将来のOTel等に繋げやすくする

---


## 18. Ports 最小契約（v2で確定）
この章は「差し替え可能境界」と「v2で必要な最小責務」を **契約**として固定する。署名は調整可能だが、以下の責務粒度は維持する。

### 18.1 TaskStore（PostgreSQL, source of truth）
- create（job/task/deps）
- claim（lease発行、attempt開始に必要な情報の取得）
- complete（状態更新・attempt/decision履歴・依存解放・ready化・outbox生成まで **同一TX**）
- outbox pull/ack/fail（PG→配送の確実化）
- reap_expired_leases（期限切れleaseの回収と再実行の起点）
- update_payload（repairタスクが payload を差し替える）

> v2 では `evaluate_readiness()` のような “外から ready を再計算するAPI” を最小に保ち、  
> **ready化は create/complete/reap の中で完結**して outbox を積む。

### 18.2 DeliveryQueue（Redis / InMemory）
- push/pop のみ（task_id を運ぶ）
- InMemory と Redis の実装差し替えが可能であること

### 18.3 ArtifactStore（Blob: MinIO/S3/Local）
- put/get/delete の最小操作を提供
- TTL は store 側の機能に依存せず、PGの `artifacts.expires_at` と GC ループで管理する（v2の簡素化）

### 18.4 RepairHintGenerator（拡張点）
- decode失敗を復旧するためのヒント生成の拡張点
- v2 は noop 実装でも成立させる（将来、エージェント実装に差し替え可能）

## 18. Ports（trait署名）の最小案（v2）
> 実装言語は Rust を想定。I/Oのため async を推奨。署名は調整可だが概念粒度は維持する。

```rust
// =========================
// TaskStore (Postgres, source of truth)
// =========================
#[async_trait::async_trait]
pub trait TaskStore: Send + Sync {
    async fn create_job(&self, ns: &str, spec: CreateJobSpec) -> Result<JobId, StoreError>;
    async fn create_task(&self, ns: &str, spec: CreateTaskSpec) -> Result<TaskId, StoreError>;
    async fn add_dependency(&self, ns: &str, task: TaskId, depends_on: TaskId) -> Result<(), StoreError>;

    // "仕事を引き受ける" (正本で lease を発行)
    async fn claim(
        &self,
        ns: &str,
        task_id: TaskId,
        worker_id: &str,
        lease_ttl: std::time::Duration,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<(Lease, TaskEnvelope)>, StoreError>;

    // 結果を確定（状態・履歴・依存解放・outbox生成まで同一TX）
    async fn complete(
        &self,
        ns: &str,
        lease: Lease,
        outcome: Outcome,
        decision: Decision,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<CompleteResult, StoreError>;

    // ready 判定（外部イベントで「もう一度評価して良い」時に呼ぶ）
    async fn evaluate_readiness(
        &self,
        ns: &str,
        task_id: TaskId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, StoreError>; // true if became ready and outbox created

    // lease 回収（期限切れを戻して再評価）
    async fn reap_expired_leases(
        &self,
        ns: &str,
        now: chrono::DateTime<chrono::Utc>,
        limit: usize,
    ) -> Result<Vec<TaskId>, StoreError>;

    // payload repair（repairタスクが実行）
    async fn update_payload(
        &self,
        ns: &str,
        task_id: TaskId,
        new_payload_artifact: ArtifactId,
        new_schema_version: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), StoreError>;

    // outbox
    async fn pull_outbox(
        &self,
        ns: &str,
        now: chrono::DateTime<chrono::Utc>,
        limit: usize,
    ) -> Result<Vec<OutboxEvent>, StoreError>;

    async fn ack_outbox(&self, ns: &str, event_id: EventId, now: chrono::DateTime<chrono::Utc>)
        -> Result<(), StoreError>;

    async fn fail_outbox(
        &self,
        ns: &str,
        event_id: EventId,
        error: String,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), StoreError>;
}

// =========================
// DeliveryQueue (Redis / InMemory)
// =========================
#[async_trait::async_trait]
pub trait DeliveryQueue: Send + Sync {
    async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError>;
    async fn pop(&self, ns: &str, timeout: std::time::Duration) -> Result<Option<TaskId>, QueueError>;
}

// =========================
// ArtifactStore (MinIO/S3/Local)
// =========================
#[async_trait::async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn put(&self, ns: &str, bytes: bytes::Bytes, content_type: Option<&str>, ttl: Option<std::time::Duration>)
        -> Result<ArtifactHandle, ArtifactError>;
    async fn get(&self, ns: &str, artifact: ArtifactId) -> Result<bytes::Bytes, ArtifactError>;
    async fn delete(&self, ns: &str, artifact: ArtifactId) -> Result<(), ArtifactError>;
}

// =========================
// RepairHintGenerator
// =========================
#[async_trait::async_trait]
pub trait RepairHintGenerator: Send + Sync {
    async fn hint(&self, input: RepairHintInput) -> Result<RepairHint, RepairError>;
}
```

---

## Appendix A. v2 実装計画（2週間 / PR分割）
> PR単位。途中で v3 送りにしても良いが、**v2のDoD（後述）**を満たすことを優先。

### Week 1：骨格と Typed API（正しさの土台）
1. **module tree 移行**（domain/ports/app/typed）
2. **ULID newtypes + IdGenerator/Clock**
3. **Typed Task API**（Task/Handler/TypedRegistry/DynHandler）
4. **DispatchStrategy（trait）+ DirectDispatch**
5. **起動時検証**（expect_tasks）
6. **InMemoryDeliveryQueue**（開発時に全体が動く）

### Week 2：PG正本 + outbox + Redis配送 + Blob/TTL + repair
7. **Postgres TaskStore + migrations**（最小スキーマ）
8. **OutboxPublisherLoop**（PG→Redis）
9. **RedisDeliveryQueue**（task_idのみ）
10. **WorkerLoop**（pop→claim→handle→decide→complete）
11. **LeaseReaperLoop**（期限切れ回収→再評価→outbox）
12. **ArtifactStore（MinIO/Local）+ artifactsメタ + TTL/GC**
13. **Decode失敗→repairタスク→payload更新→再実行**
14. **docker compose（全部入り）+ 統合テストシナリオ**

---

## 19. v2 DoD（完了条件）
- PGが task/job の状態・履歴・依存・outbox を保持できる
- outbox publisher が ready task を DeliveryQueue に配送できる
- worker が DeliveryQueue から task_id を取り、PGで claim → handler実行 → complete できる
- lease expiry が回収され再配送される（at-least-once 成立）
- typed task API で task_type typo を排除し、起動時検証で未登録が検知できる
- decode失敗が repair 経由で復旧でき、max_repairs で無限ループを防げる
- DAG依存で downstream が ready 化され配送される
- docker compose で PG/Redis/Worker/Publisher/Blob が立ち上がり、統合シナリオが動く

---

## 20. v3 バックログ（将来）

### 21.1 Dispatch高度化
- RuleDispatch（task_type + メタ情報）
- AgentDispatch（AI/エージェント）
- ルーティング理由の保存（Decisionの説明可能性）

### 21.2 compile-time 完全性保証
- typestate builder / マクロで「全タスク登録済み」しか build できない

### 21.3 バックプレッシャ / レート制御
- queue長やjob単位の同時実行制限、生成と実行の分離

### 21.4 さらに強い整合性（Inbox 等）
- outboxに加えて inbox（受信側 dedupe）導入
- exactly-once “風” の強化（ただしコストに注意）

### 21.5 イベントソーシング寄り
- events 正本 + projection / snapshot / replay ツール

### 21.6 Kafka / Redpanda への配送層移行
- DeliveryQueue を Kafka/Redpanda adapter に置換
- outbox publisher を Kafka producer に変更
- consumer group / offset の運用を取り込む

#### Kafka / Redpanda（簡単な説明）
- Kafka: 分散ログ（追記ログ）を中心にしたストリーミング基盤。リプレイが得意。
- Redpanda: Kafka互換APIの基盤。運用簡素化・軽量化の文脈で代替になり得る。
- Weaverでは「PG正本 + outbox + 配送アダプタ」を守れば、Redis→Kafka/Redpandaへ移行しやすい。

### 21.7 Artifact高度化
- retention policy、圧縮・差分・参照カウント、ベクトルストア統合

### 21.8 cancel 強制停止（危険領域）
- サンドボックス/プロセス分離、kill、補償（saga）など

---

## 21. 議論外だが要件に効く提案（追加）

### 22.1 namespace（ベストプラクティス）
- config で `namespace` 必須（例: `acme-dev`, `acme-prod`）
- PG全テーブルに namespace を含める
- Redis キー prefix も namespace で分離
- task_type の namespace（命名規約）とは別物（運用環境分離）

### 22.2 “巨大データ禁止”を契約化
- payload直埋めの上限を決め、超過は artifact へ強制

### 22.3 エラー分類を固定（運用）
- `error_kind` を enum 的に揃える（decode_error/handler_error/budget/deadline/cancel/lease_expired など）
- waiting_reason で「なぜ進んでないか」を必ず説明可能にする

---
