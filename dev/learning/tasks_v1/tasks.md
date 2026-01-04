# Weaver 学習タスク

このファイルは全フェーズのタスクリストを管理します。
日々の実装記録は日付付きファイル（`YYYY_MM_DD.md`）を参照してください。

---

## 📊 進捗サマリー

- ✅ Phase 1: 基礎実装（完了）
- ✅ Phase 2: Job-level Abstraction（完了）
- ✅ Phase 3: Attempt/Decision 記録（完了）
- ✅ Phase 4-1: Decider 統合（完了）
- ✅ Phase 4: Task 分解（完了）
- ⏳ Phase 5: 依存関係管理
- ⏳ Phase 6: Budget と Stuck 検知
- ⏳ Phase 7: API の実装
- ⏳ Phase 8: Artifact の実装

---

## Phase 1: 基礎実装 ✅ 完了

基本的なタスク実行とリトライの仕組み。

- [x] Domain model (IDs, Specs, Outcomes) の実装
- [x] Error types の定義
- [x] Queue trait + InMemoryQueue の実装
- [x] TaskLease, TaskRecord, TaskState の実装
- [x] RetryPolicy の実装
- [x] HandlerRegistry + Runtime の実装
- [x] Worker/WorkerGroup の実装
- [x] 基本的なタスク実行とリトライ機能

**完了日**: 2025-12-28
**学習記録**: `dev/learning/learning_2025_12_28.md`

---

## Phase 2: Job-level Abstraction ✅ 完了

Task 単位から Job（複数タスクの集合）単位への拡張。

- [x] JobRecord の実装（複数 Task を含む）
- [x] JobState の実装と状態集約ロジック
- [x] TaskRecord への job_id 追加
- [x] JobId による管理機能（CRUD）
- [x] Job → Task の関連付け（create_job_with_tasks）
- [x] Job 全体のステータス集約（update_state_from_tasks）
- [x] submit_job API の実装

**ゴール**: ✅ `submit_job(JobSpec) -> JobId` API 実装完了
**完了日**: 2025-12-29
**学習記録**: `dev/learning/learning_2025_12_29.md`

---

## Phase 3: Attempt/Decision の記録 ✅ 完了

実行履歴と判断の記録を残す仕組み。

- [x] AttemptRecord 構造体の定義
- [x] DecisionRecord 構造体の定義
- [x] AttemptRecord::new コンストラクタ
- [x] DecisionRecord::new コンストラクタ
- [x] InMemoryQueueState への統合
  - [x] attempts HashMap の追加
  - [x] decisions Vec の追加
  - [x] allocate_attempt_id メソッド
- [x] TaskLease での AttemptRecord 記録
  - [x] ack() での成功記録
  - [x] fail() での失敗記録
- [x] リトライ判断時の DecisionRecord 記録
  - [x] mark_dead パスでの記録
  - [x] schedule_retry パスでの記録

**ゴール**: 「なぜこの結果になったか」を説明可能にする ✅ 達成
**完了日**: 2025-12-30
**学習記録**: `dev/learning/learning_2025_12_29.md`, `dev/learning/learning_2025_12_30.md`

**注**: 履歴取得 API と Job レベルでの履歴集約は Phase 7 で実装予定

---

## Phase 4-1: Decider 統合 ✅ 完了

Handler → Outcome → Decider → Decision フローを実行エンジンに統合する。

### 完了（全ステップ）
- [x] Step 1: TaskLease Interface の拡張
  - [x] `get_task_record()` メソッド追加
  - [x] `complete(outcome, decision)` メソッド追加
- [x] Step 2: Handler Trait の変更
  - [x] `handle()` の戻り値を `Result<Outcome, WeaverError>` に変更
  - [x] Runtime::execute() の更新
- [x] Step 3: Decider を Worker に統合
  - [x] WorkerGroup::spawn() に decider パラメータ追加
  - [x] Decider trait に Send + Sync 追加
- [x] Step 4: Worker Loop Flow の実装
  - [x] Handler → Outcome → Decider → Decision のフロー実装
  - [x] SUCCESS 時の最適化（Decider バイパス）
  - [x] インフラエラーの Outcome 変換
- [x] Step 5: get_task_record() の実装
  - [x] InMemoryLease::get_task_record() 実装
- [x] Step 6: complete() の実装
  - [x] AttemptRecord 作成と挿入
  - [x] Decision に基づく分岐（Retry/MarkDead）
  - [x] ADR-0003 準拠（lock-outside-notify）
- [x] Step 6.5: complete() の単体テスト
  - [x] Retry decision パスのテスト
  - [x] MarkDead decision パスのテスト
  - [x] レコード作成の検証

### 完了（続き）
- [x] Step 7: Handler 更新（CLI の HelloHandler を新パターンに変換）
  - [x] HelloHandler を `Result<Outcome, WeaverError>` に更新
  - [x] main 関数で DefaultDecider を作成
  - [x] 動作確認（`cargo run -p weaver-cli`）
- [x] Step 8: 統合テスト
  - [x] test_worker_retry_flow_integration（リトライフロー全体）
  - [x] test_worker_max_attempts_exceeded（max_attempts 超過）
  - [x] test_worker_immediate_success（即座に成功）
- [x] Step 9: CLI 動作確認（Step 7 で実施）
- [x] Step 10: ドキュメント更新
  - [x] ADR-0005 を "Accepted" に更新
  - [x] learning 記録の最終更新

**ゴール**: ✅ 純粋関数（Decider）と副作用（Worker/TaskLease）の分離、カスタマイズ可能な判断ロジック
**開始日**: 2026-01-01
**完了日**: 2026-01-02
**学習記録**: `dev/learning/learning_2026_01_01.md`, `dev/learning/learning_2026_01_02.md`
**関連 ADR**: `dev/docs/adr/0005-decider-architecture.md` (Accepted)

**テスト結果**: 全31テストパス（単体テスト + 統合テスト）

---

## Phase 4: Task 分解（Decomposition） ✅ 完了

抽象的/大きすぎるタスクを実行可能単位に分解する。

### 完了（全ステップ）
- [x] Step 1: TaskState に Decomposed を追加
  - [x] TaskState enum に Decomposed variant 追加
  - [x] counts_by_state() でのカウント対応
- [x] Step 2: Decision に Decompose variant を追加
  - [x] Decision::Decompose { child_tasks, reason } の定義
  - [x] TaskSpec のクローン可能性確保
- [x] Step 3: Outcome に child_tasks フィールドを追加
  - [x] Outcome::child_tasks: Option<Vec<TaskSpec>> 追加
  - [x] with_decompose_hint() ヘルパーメソッド実装
- [x] Step 4: TaskRecord に親子関係フィールドを追加
  - [x] parent_task_id: Option<TaskId> フィールド追加
  - [x] child_task_ids: Vec<TaskId> フィールド追加
  - [x] new_child() コンストラクタ追加
- [x] Step 5: TaskSpec に task_type と payload を追加（設計改善）
  - [x] TaskSpec に task_type, payload フィールド追加
  - [x] new() コンストラクタのシグネチャ更新
  - [x] 既存テストの更新
- [x] Step 6: add_child_tasks() メソッドの実装とテスト
  - [x] TaskLease trait に add_child_tasks() 追加
  - [x] InMemoryLease での実装
  - [x] Lock 最小化パターン適用（ADR-0003 準拠）
  - [x] 単体テスト作成
- [x] Step 7: Decision::Decompose の処理実装
  - [x] complete() に Decompose ブランチ追加
  - [x] add_child_tasks() 呼び出し
  - [x] 親タスクを Decomposed に遷移
  - [x] DecisionRecord 記録
- [x] Step 8: Decider が child_tasks を考慮するよう更新
  - [x] DefaultDecider::decide() 更新
  - [x] child_tasks 優先順位を最上位に
  - [x] if-let パターン活用
- [x] Step 9: 統合テストと動作確認
  - [x] DecomposingHandler 実装（テスト用）
  - [x] test_task_decomposition_integration 作成
  - [x] End-to-End 動作確認
  - [x] バグ修正（submit_job, Worker Success ハンドリング）

**ゴール**: ✅ 大きなタスクを自動的に小さな実行単位に分解
**開始日**: 2026-01-02
**完了日**: 2026-01-03
**学習記録**: `dev/learning/learning_2026_01_02.md`, `dev/learning/learning_2026_01_03.md`

**テスト結果**: 全33テストパス（Phase 4 の統合テスト含む）

**主要な学び**:
- Lock 最小化パターン（3-phase: acquire → process → update）
- Handler → Decider → Worker の責務分離
- Success でも child_tasks がある場合は Decider フローに進む設計
- End-to-End テストでバグを発見する重要性

---

## Phase 5: 依存関係管理 ⏳ 未着手

タスク間の「これが終わらないと進めない」関係を表現する。

- [ ] Dependency モデルの実装
  - [ ] 依存関係の表現（TaskId → TaskId）
  - [ ] 依存タイプの定義（必須/推奨など）
- [ ] 依存グラフの管理
  - [ ] グラフ構造の保持
  - [ ] 依存関係の追加/削除
- [ ] 依存解決のスケジューリング
  - [ ] 依存先が完了したタスクを ready に昇格
  - [ ] 実行可能タスクの判定
- [ ] 循環依存の検出
  - [ ] グラフの巡回検出
  - [ ] エラーハンドリング

**ゴール**: タスクが依存関係を持てるようにし、自動的に順序制御

---

## Phase 6: Budget と Stuck 検知 ⏳ 未着手

実行制約と「進めない状態」の検知。

- [ ] Budget の実装
  - [ ] max_attempts（既に RetryPolicy で部分的に実装済み）
  - [ ] deadline（期限）の実装
  - [ ] max_total_cost（コスト上限）の実装
  - [ ] Budget 超過の検出
- [ ] Stuck 検知ロジック
  - [ ] RUNNABLE が存在しない状態の検出
  - [ ] 依存サイクルの検出（Phase 5 と連携）
  - [ ] Budget 到達の検出
  - [ ] 無限ループの防止
- [ ] 適切な終了処理
  - [ ] Stuck 時の Job 状態遷移
  - [ ] 部分完了の記録

**ゴール**: 無限ループを防ぎ、適切なタイミングで終了

---

## Phase 7: API の実装 ⏳ 未着手

外部から利用可能な API を整備する。

- [x] `submit_job(JobSpec) -> JobId` ✅ Phase 2 で完了
- [ ] `get_status(JobId) -> JobStatus`
  - [ ] Job 状態の取得
  - [ ] Task 状態の集約
  - [ ] 進捗情報の提供
- [ ] `cancel_job(JobId) -> CancelAck`
  - [ ] 実行中 Job のキャンセル
  - [ ] 実行中 Task の停止
  - [ ] クリーンアップ処理
- [ ] `get_result(JobId) -> JobResult`
  - [ ] 完了 Job の結果取得
  - [ ] 部分完了の場合の処理
  - [ ] Attempt/Decision 履歴の取得

**ゴール**: ライブラリとして使いやすい API を提供

---

## Phase 8: Artifact の実装 ⏳ 未着手

実行結果の成果物を記録・参照する。

- [ ] Artifact の保存機構
  - [ ] ファイルの保存
  - [ ] URL の記録
  - [ ] stdout/stderr の記録
- [ ] Artifact の取得 API
  - [ ] TaskId による取得
  - [ ] JobId による一括取得
- [ ] ストレージの抽象化
  - [ ] ファイルシステム
  - [ ] 将来の拡張性（S3 等）

**ゴール**: 実行結果の成果物を追跡可能に

---

## 📚 参考資料

- **要件**: `dev/docs/requirements/2025_12_27_weaver_requirements.md`
- **アーキテクチャ**: `CLAUDE.md`
- **ADR**: `dev/docs/adr/`
- **日々の実装記録**: `dev/learning/learning_YYYY_MM_DD.md`
