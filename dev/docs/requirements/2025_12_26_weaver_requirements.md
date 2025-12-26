# Rust タスクキュー v1 要件定義（Draft）

## 目的
1プロセス内で動作する **インメモリのタスクキュー**を Rust で実装する。  
タスクはキューに入り、ワーカーが取り出して実行する。失敗時は **遅延ありのリトライ**を行う。  
将来的にキュー実装は差し替え可能（インメモリDB風・永続化DB等）な構造を目指す。

---

## スコープ
### v1でやること
- **1プロセス**で動作
- **tokio async worker** によりタスクを実行
- キューは **インメモリ実装**
- タスク表現は **handler 登録制（TaskType + Payload）**
- payload は **serde_json（bytes）**
- **失敗時のリトライ**（最大 5 回、遅延あり）
  - リトライ対象は必ず「キューに戻す」方式（sleepしながらその場でリトライしない）
- **状態の保持と観測**
  - `task_id` で状態を取得できる
  - 状態別の件数（例：Queuedが何件、RetryScheduledが何件）も取得できる

### v1でやらないこと（将来の拡張）
- 複数プロセス/分散実行
- 並行度の高度な制御（同一キー直列化、依存関係DAG、優先度など）
- lease/可視タイムアウト/ワーカー落ち復旧
- DLQの永続化（Dead状態は持つが、保管や再投入UXはv2以降）

---

## 基本コンセプト（Rustっぽい設計方針）
- **データと挙動を分離**
  - キューが持つのは「実行可能なデータ（Envelope）＋メタデータ」
  - 実行ロジックは handler（trait実装）側に集約
- **trait によるポート定義**
  - `Queue` は trait として定義し、実装（InMemory/永続化）を差し替え可能にする
  - `TaskHandler` は trait として定義し、タスクを登録（registry）して実行時に解決する
- **状態は明示的に持つ（状態機械）**
  - 成功/失敗/リトライスケジュール/最終失敗（Dead）を状態として保持し、観測可能にする

---

## タスク表現
### TaskEnvelope（キューに入る“タスク本体データ”）
- `task_type: &'static str`（例: `"send_email"`）
- `payload: Vec<u8>`（serde_json の bytes）
- `content_type`（v1はJSON固定でも可）

### TaskHandler（登録して実行する“挙動”）
- `task_type()` で自分が扱う type を名乗る
- `execute(payload_bytes)` で payload を decode（serde_json）して処理する
- `execute` の結果が `Ok` なら成功、`Err` なら v1は常にリトライ対象（上限あり）

---

## キュー内データモデル
### TaskRecord（キューが保持するメタデータ付きレコード）
- `id: TaskId`
- `envelope: TaskEnvelope`
- `state: TaskState`
- `attempts: u32`（これまでの失敗回数 / 実行回数の定義は実装で統一）
- `max_attempts: u32`（v1は 5 固定）
- `last_error: Option<String>`
- `next_run_at: Option<Instant>`（リトライ遅延用）
- `created_at / updated_at`（観測用）

### TaskState（v1最小セット）
- `Queued`（即時実行待ち）
- `Running`（実行中）
- `Succeeded`（成功）
- `RetryScheduled`（遅延リトライ待ち：`next_run_at`まで待機）
- `Dead`（試行上限超過で停止）

---

## リトライ仕様
- 最大試行回数：**5回**
- 遅延あり（backoff）
  - v1では固定でも指数でも良いが、差し替え可能な `RetryPolicy` として表現する
- v1では **すべてのエラーはリトライ対象**
- `attempts` が上限を超えたら `Dead` へ遷移し、キューから再実行対象としては出てこない

---

## キューのデータ構造（InMemoryQueue v1）
- `records: HashMap<TaskId, TaskRecord>`（id -> レコード）
- `ready: VecDeque<TaskId>`（即時実行待ち）
- `scheduled: （next_run_at順に取り出せる構造）`（例: BinaryHeap + Reverse、または BTreeMap）
- `notify: tokio::sync::Notify`（enqueue/スケジュール更新でワーカーを起こす）

方針：
- 実際の待ち行列は **TaskIdのみ**
- 実体（Envelope/状態/attemptsなど）は `records` に集約

---

## 実行モデル（Worker）
- ループで `dequeue_ready()` し、得られた `task_id` の task_type に対応する handler を解決して実行
- 成功なら `mark_succeeded(id)`
- 失敗なら `mark_failed(id, err)`（Queue側で retry schedule / dead 判定し、必要なら再投入）

---

## 観測（Observability）要件
最低限：
- `get_status(task_id)`：状態、attempts、last_error、next_run_at などが取得できる

追加（v1で欲しい）：
- `counts_by_state()`：状態別の件数（Queued/Running/Succeeded/RetryScheduled/Dead）

（将来）
- state遷移イベントの購読（broadcast/watch）
- 一覧取得（state filter, pagination）

---

## 差し替え（将来要件）
- `Queue` は trait として抽象化する
- InMemoryQueue以外（インメモリDB風/永続化DB）に差し替えても
  - `TaskEnvelope` と `TaskRecord` の概念が維持される
  - handler登録制（task_type + payload）で動作する

---

## 将来構想 / 設計原則（v1 から意図として明文化）

本システムにおける「キュー」は一部分であり、最終的には **タスク実行ランタイム（Job Runtime）**として “実行・再実行・観測” をうまく扱うことを目的とする。  
v1 は 1プロセス + インメモリで開始するが、設計としては将来の拡張（永続化キュー、外部キュー、オーケストレーション、LLM/Agent ワーカー）を阻害しない。

### 将来構想（方向性）
- **より堅牢なキューへの差し替え**  
  Redis や他のキューシステム、あるいは永続化されたキューデータベースへ差し替え可能な境界を維持する。
- **ワークフロー / オーケストレーション**  
  Temporal のようなワークフロー的機能は将来の構想に含む。  
  ただし同一ソフトウェア内で完結させることは必須としない（外部の orchestrator が enqueue する形でもよい）。
- **LLM / AI Agent ワーカーとの相性**  
  LLM/Agent を worker として動作させられること（= “タスク＝データ” を中心に設計できること）を重視する。

---

## Tokio の位置づけと非同期方針（v1）

- v1 は **tokio ベースの async 実行モデル**で実装する。
- 非同期設計のガードレール（実装規約）：
  - **ロックを保持したまま `.await` しない**  
    Queue 内部の排他は短く保ち、実行（handler 呼び出し）はロック外で行う。
  - ブロッキング処理（重い CPU 計算や同期 I/O）が混ざる場合は **`spawn_blocking` 等に隔離**する。
  - **キャンセル（タスク中断）され得る**前提で、状態遷移が壊れないようにする（例：Running のまま取り残さない）。

---

## ワーカー実行フロー（状態遷移とロック境界）

既存の実行モデル（`dequeue_ready()` → handler 実行 → `mark_*`）を、事故を減らすために **実装規約として具体化**する。

### 推奨フロー
1. `dequeue_ready()` で `task_id` を取得
2. `records[task_id]` を参照し、実行に必要な `TaskEnvelope` を **ロック内で取り出して（clone など）** state を Running に更新
3. **ロックを解放**して handler を実行（`.await` はここ）
4. 結果に応じて **再度ロック**し、Succeeded / RetryScheduled / Dead へ遷移し、必要なら `ready/scheduled` を更新

---

## 「キューには TaskId のみ」を設計方針として固定（理由の明記）

### 方針
- 待ち行列は **TaskId のみ**を保持し、実体（Envelope/状態/attempts/エラーなど）は `records` に集約する。

### 理由（利点）
- 状態（Queued/Running/...）と観測 API（`get_status`, `counts_by_state`）が **単一の正本（records）に集約**される。
- キュー操作が軽い（payload が大きくてもキューが肥大化しにくい）。
- Rust/async 的に「参照をキューに保持する」設計を避けられ、**所有権・ライフタイム・ロック跨ぎ await**の難しさを抑えられる。

### 実体を丸ごとキューに入れる案について（非推奨理由）
- いずれ `records`（状態管理）も必要になりがちで **二重管理**になりやすい。
- mutable 状態（attempts / next_run_at / last_error）を「どこが正本か」曖昧にしやすい。

---

## 遅延リトライ（scheduled）の実装選択肢と不変条件

### 選択肢
- **BinaryHeap + Reverse（優先度付きキュー）**  
  シンプル。`next_run_at` 最小を取り出す用途に直結する。
- **tokio-util DelayQueue**  
  タイマー駆動で「期限が来たものを取り出す」を自然に書ける（依存追加と扱いに慣れが必要）。

### 満たすべき不変条件
- `RetryScheduled` の task は `next_run_at` まで `ready` に出てこない。
- enqueue / schedule 更新時には `notify` でワーカーを起こす。

---

## 観測（Observability）の最低限ログ（推奨）

既存の観測要件（`get_status`, `counts_by_state`）に加え、最低限のイベントログを出すことを推奨する（構造化ログ推奨）。

- Enqueued（task_id, task_type）
- Started（task_id, attempt）
- Succeeded / Failed（task_id, attempt, err）
- RetryScheduled（task_id, next_run_at, backoff）
- Dead（task_id, last_error, attempts）

---

## 未決定項目（v1 で決める/決めないを明示推奨）

実装のブレを防ぐため、以下は「v1では決める／決めない」を明記して管理する。

1. **並行度**
   - ワーカーは 1 本か、複数タスクで並列実行するか（最大並行数）
   - handler ごとの並行上限（v1 では不要なら“やらない”と明記）

2. **attempts の定義**
   - `attempts` が「失敗回数」なのか「実行回数」なのか（v1 で固定推奨）
   - `max_attempts=5` の意味（初回含めて5回？失敗が5回？）

3. **backoff の具体**
   - 固定 / 指数 / jitter ありなど、v1 のデフォルト
   - `RetryPolicy` のインターフェース（入力：attempts？エラー種別？）

4. **時間の型**
   - v1 は `Instant` を使う前提でよい（インメモリのため）。
   - v2 の永続化に備えるなら、`SystemTime`/epoch などへ移行が必要になり得る旨を注意点として記載する。

5. **シャットダウンとキャンセル**
   - graceful shutdown（処理中タスクの扱い）
   - Running 取り残し対策（v1 は“落ちたら全部消える”でも、状態として矛盾しない設計が必要）

6. **メモリ保持方針**
   - Succeeded/Dead の TaskRecord をいつ捨てるか（無制限だとメモリが増え続ける）
   - purge API や TTL（v1で不要なら “やらない” と明記）

7. **実行保証（少なくとも1回 / 高々1回）**
   - v1 は単一プロセスでも、失敗・再投入があるため **at-least-once** 寄りになり得る。
   - 重複実行の扱い（許容する/しない/後で考える）を明記する。

8. **エラー分類**
   - v1 は「すべてリトライ対象」だが、payload 不正など “永久に治らない” エラーの扱いは v2 で検討対象として残す。

