# Weaver 学習タスク

このファイルは全フェーズのタスクリストを管理します。
日々の実装記録は日付付きファイル（`YYYY_MM_DD.md`）を参照してください。

---

## 📊 進捗サマリー

- ✅ Phase 1: 基礎実装（完了）
- ✅ Phase 2: Job-level Abstraction（完了）
- ✅ Phase 3: Attempt/Decision 記録（実装部分完了）
- ⏳ Phase 4: Task 分解
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
**学習記録**: `dev/learning/2025_12_28.md`

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
**学習記録**: `dev/learning/2025_12_29.md`

---

## Phase 3: Attempt/Decision の記録 ✅ 完了（実装部分）

実行履歴と判断の記録を残す仕組み。

### 完了
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

### 未完了（将来的に Phase 7 で実装）
- [ ] 履歴取得 API
- [ ] Job レベルでの履歴集約

**ゴール**: 「なぜこの結果になったか」を説明可能にする ✅ 達成
**完了日**: 2025-12-30
**学習記録**: `dev/learning/2025_12_29.md`, `dev/learning/2025_12_30.md`

---

## Phase 4: Task 分解（Decomposition） ⏳ 未着手

抽象的/大きすぎるタスクを実行可能単位に分解する。

- [ ] Decider trait の設計
  - [ ] 純粋関数として実装（副作用の分離）
  - [ ] 入出力の型定義
- [ ] 分解ロジックの実装
  - [ ] TaskSpec → 複数の TaskSpec への分解
  - [ ] 分解条件の判定
- [ ] 親子関係の管理
  - [ ] 親タスクと子タスクの関連付け
  - [ ] 子タスクの進捗追跡
- [ ] 子タスク完了時の親タスク処理
  - [ ] 全子タスク成功 → 親タスク成功
  - [ ] いずれか失敗 → 親タスクの判断

**ゴール**: 大きなタスクを自動的に小さな実行単位に分解

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
- **日々の実装記録**: `dev/learning/YYYY_MM_DD.md`
