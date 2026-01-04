# Weaver v2 学習タスク

このファイルは **v2 実装**の全タスクを管理します。
日々の実装記録は日付付きファイル（`learning_YYYY_MM_DD.md`）を参照してください。

**v1 のタスク**: `tasks_v1/tasks.md` を参照

---

## 📊 進捗サマリー

### v2 実装計画（2週間）

- ⏳ Week 1: 骨格と Typed API（正しさの土台）
- ⏳ Week 2: PG正本 + outbox + Redis配送 + Blob/TTL + repair

### 完了条件（v2 DoD）

v2 が完成したと言えるための条件：

- [ ] PGが task/job の状態・履歴・依存・outbox を保持できる
- [ ] outbox publisher が ready task を DeliveryQueue に配送できる
- [ ] worker が DeliveryQueue から task_id を取り、PGで claim → handler実行 → complete できる
- [ ] lease expiry が回収され再配送される（at-least-once 成立）
- [ ] typed task API で task_type typo を排除し、起動時検証で未登録が検知できる
- [ ] decode失敗が repair 経由で復旧でき、max_repairs で無限ループを防げる
- [ ] DAG依存で downstream が ready 化され配送される
- [ ] docker compose で PG/Redis/Worker/Publisher/Blob が立ち上がり、統合シナリオが動く

---

## 🎯 v2 の目標と非目標（確認用）

### v2 の目標（MUST）

1. **PG正本 + outbox** により状態・履歴・依存・配送指示が確実に残る
2. **Redis配送**（または InMemory配送）によりワーカーが task_id を受け取って実行できる
3. **Typed Task API**で task_type typo を排除し、**起動時検証**で登録漏れを検知できる
4. **Lease + visibility timeout** によりワーカー死活に強く、at-least-once を成立させる
5. **Decode失敗の復旧フロー**（repair task + hint）と **無限ループ防止**
6. **Artifact/TTL/GC** により巨大コンテキストを扱える
7. **docker compose** により統合動作が再現できる（ローカルで検証可能）

### v2 の非目標（明示）

- データ保護（暗号化・マスキング・アクセス制御）は **扱わない**
- バックプレッシャ/レート制御は **扱わない**（先で検討）
- 強制 cancel（kill/強制中断）は **扱わない**（best-effort のみ）
- "全タスク登録済み"を型レベルで保証（compile-time完全性）は **扱わない**（起動時検証で十分）
- イベントソーシング（events正本 + projection）は **採用しない**（現状態 + 履歴）

---

## Week 1: 骨格と Typed API（正しさの土台）

**ゴール**: PG/Redis なしでも動く骨格と、型安全な Task API の完成

### PR-1: Module Tree 移行 ⏳ 未着手

**目的**: v2 のアーキテクチャに合わせた module 構成に再編成する

- [ ] weaver-core の module tree を v2 仕様に再構成
  - [ ] `domain/` モジュール（ids, task_type, envelope, budget, outcome, decision, state, errors, events）
  - [ ] `ports/` モジュール（task_store, delivery_queue, artifact_store, decider, dispatch, repair_hint, clock, id_generator, event_sink）
  - [ ] `app/` モジュール（builder, runtime, worker_loop, publisher_loop, reaper_loop, gc_loop, status）
  - [ ] `typed/` モジュール（task, handler, registry, codec）
  - [ ] `impls/` モジュール（inmem_delivery）
- [ ] v1 のコードとの互換性を一時的に維持（段階的移行）
- [ ] ビルドが通ることを確認（`cargo check`）

**学習ポイント**:
- Rust のモジュールシステム（pub mod, pub use）
- 責務の分離（domain, ports, app, typed の役割）
- Hexagonal Architecture（ポート&アダプタ）の実践

**参考**: 要件ドキュメント 3.2 節「推奨 module tree」

---

### PR-2: ULID Newtypes + IdGenerator/Clock ⏳ 未着手

**目的**: 分散システムで使える ID 生成と時刻抽象化

- [ ] ULID newtypes の実装（JobId, TaskId, AttemptId, etc.）
  - [ ] ulid クレート導入
  - [ ] newtype パターンで型安全性確保
  - [ ] Serialize/Deserialize 実装
- [ ] IdGenerator trait の定義
  - [ ] generate_job_id(), generate_task_id() など
  - [ ] UlidGenerator 実装（デフォルト）
- [ ] Clock trait の定義
  - [ ] now() メソッド
  - [ ] SystemClock 実装（デフォルト）
  - [ ] FixedClock 実装（テスト用）
- [ ] テスト作成（ULID の単調増加性など）

**学習ポイント**:
- ULID の特性（時刻ソート可能、分散生成可能）
- Trait による依存性注入（テスト容易性）
- newtype パターンによる型安全性

**参考**: 要件ドキュメント 3.1 節「domain/ids.rs」

---

### PR-3: Typed Task API（Task/Handler/TypedRegistry/DynHandler） ⏳ 未着手

**目的**: task_type の typo を型で排除し、Handler との対応付けを静的に保証

- [ ] Task trait の定義
  - [ ] `const TYPE: &'static str;` 定義
  - [ ] task_type と型の対応付け
- [ ] Handler trait の定義
  - [ ] `async fn handle(&self, task: T) -> Result<Outcome, WeaverError>`
  - [ ] ジェネリクスで Task と Handler を結びつける
- [ ] TypedRegistry の実装
  - [ ] `register::<T: Task>(handler: impl Handler<T>)` メソッド
  - [ ] 内部的に DynHandler に変換
  - [ ] HashMap<String, Arc<dyn DynHandler>> で管理
- [ ] DynHandler trait の実装
  - [ ] object-safe な trait（deserialize + handle）
  - [ ] TypedHandler<T> → DynHandler adapter
- [ ] PayloadCodec の実装
  - [ ] artifact → T のデシリアライズ
  - [ ] T → artifact のシリアライズ
- [ ] テスト作成（型安全性の確認）

**学習ポイント**:
- Rust の const generic / associated constants
- Trait object と object safety
- Type erasure パターン（Typed → Dyn）
- ジェネリクスと型パラメータ

**参考**: 要件ドキュメント 5章「型付き Task API（二層）」

---

### PR-4: DispatchStrategy（trait）+ DirectDispatch ⏳ 未着手

**目的**: 将来の拡張（Rule/Agent dispatch）に備えた抽象化

- [ ] DispatchStrategy trait の定義
  - [ ] `fn select_handler(&self, task_type: &str, meta: &TaskMeta) -> Result<String, DispatchError>`
  - [ ] task_type → handler_id の解決
- [ ] DirectDispatch 実装
  - [ ] 1:1 マッピング（task_type == handler_id）
  - [ ] v2 のデフォルト実装
- [ ] App への統合
  - [ ] AppBuilder で strategy を差し替え可能に
  - [ ] Runtime での利用
- [ ] テスト作成（DirectDispatch の動作確認）

**学習ポイント**:
- Strategy パターン
- Trait による振る舞いの差し替え
- 将来の拡張性を考えた設計

**参考**: 要件ドキュメント 6章「DispatchStrategy」

---

### PR-5: 起動時検証（expect_tasks） ⏳ 未着手

**目的**: 登録漏れを起動時に検知し、fail-fast

- [ ] AppBuilder::expect_tasks() メソッド
  - [ ] 期待される task_type のリストを受け取る
  - [ ] build() 時に「期待集合 ⊆ 登録済み集合」をチェック
  - [ ] 不足があれば panic または Error
- [ ] テスト作成
  - [ ] 登録漏れの検出テスト
  - [ ] 正常系テスト
- [ ] エラーメッセージの改善
  - [ ] 不足している task_type を列挙

**学習ポイント**:
- Builder パターンでの検証ロジック
- Fail-fast 設計
- 開発体験の改善（明確なエラーメッセージ）

**参考**: 要件ドキュメント 5.3 節「起動時検証」

---

### PR-6: InMemoryDeliveryQueue（開発用） ⏳ 未着手

**目的**: PG/Redis なしで動作確認できる開発用キュー

- [ ] DeliveryQueue trait の定義
  - [ ] `async fn push(&self, ns: &str, task_id: TaskId) -> Result<(), QueueError>`
  - [ ] `async fn pop(&self, ns: &str, timeout: Duration) -> Result<Option<TaskId>, QueueError>`
- [ ] InMemoryDeliveryQueue 実装
  - [ ] VecDeque + Mutex/RwLock での実装
  - [ ] namespace 対応
  - [ ] blocking pop（timeout 付き）
- [ ] テスト作成（push/pop の動作確認）

**学習ポイント**:
- Trait による抽象化（Redis と差し替え可能）
- Mutex/Condvar による同期（blocking pop）
- Async での blocking 処理の扱い

**参考**: 要件ドキュメント 15章「DeliveryQueue」、18.2節「Ports 最小契約」

---

## Week 2: PG正本 + outbox + Redis配送 + Blob/TTL + repair

**ゴール**: 本番で使える永続化・配送・復旧機能の実装

### PR-7: Postgres TaskStore + migrations ⏳ 未着手

**目的**: PG を正本（source of truth）として状態・履歴・依存・outbox を管理

- [ ] TaskStore trait の定義
  - [ ] create_job / create_task / add_dependency
  - [ ] claim（lease 発行）
  - [ ] complete（状態更新・履歴記録・依存解放・outbox生成まで同一TX）
  - [ ] evaluate_readiness（ready 再評価）
  - [ ] reap_expired_leases（期限切れ回収）
  - [ ] update_payload（repair 用）
  - [ ] pull_outbox / ack_outbox / fail_outbox
- [ ] PostgreSQL スキーマ設計
  - [ ] jobs テーブル
  - [ ] tasks テーブル
  - [ ] task_dependencies テーブル
  - [ ] attempts テーブル
  - [ ] decisions テーブル
  - [ ] outbox_events テーブル
  - [ ] artifacts テーブル
- [ ] weaver-pg クレート作成
  - [ ] sqlx 導入（PostgreSQL driver）
  - [ ] migrations 管理（sqlx-cli）
  - [ ] PostgresTaskStore 実装
- [ ] トランザクション境界の明確化
  - [ ] create/complete/reap などで TX 制御
  - [ ] outbox の生成を同一 TX 内で保証
- [ ] テスト作成
  - [ ] 各メソッドの単体テスト
  - [ ] トランザクションの整合性テスト

**学習ポイント**:
- sqlx での async SQL 処理
- PostgreSQL のトランザクション管理
- Outbox パターン（配送の確実化）
- データベーススキーマ設計

**参考**: 要件ドキュメント 12章「PostgreSQL 最小スキーマ」、18.1節「TaskStore（PostgreSQL, source of truth）」

---

### PR-8: OutboxPublisherLoop（PG→Redis） ⏳ 未着手

**目的**: PG の outbox を読んで DeliveryQueue に配送する

- [ ] OutboxPublisherLoop の実装
  - [ ] `pull_outbox()` で pending イベントを取得
  - [ ] `DeliveryQueue::push()` で配送
  - [ ] `ack_outbox()` で sent にマーク
  - [ ] エラー時は `fail_outbox()` でリトライ
- [ ] ループロジック
  - [ ] 定期的にポーリング（例: 100ms）
  - [ ] バッチ処理（limit: 100）
- [ ] Graceful shutdown 対応
  - [ ] CancellationToken での停止
- [ ] テスト作成（モックでの動作確認）

**学習ポイント**:
- Outbox パターンの実装
- ポーリングループの設計
- Tokio での Graceful shutdown

**参考**: 要件ドキュメント 14章「Outbox」、3.2節「app/publisher_loop.rs」

---

### PR-9: RedisDeliveryQueue（task_idのみ） ⏳ 未着手

**目的**: Redis で task_id のみを流す配送キュー

- [ ] weaver-redis クレート作成
  - [ ] redis クレート導入
  - [ ] RedisDeliveryQueue 実装
- [ ] DeliveryQueue trait の実装
  - [ ] `push()`: RPUSH でキューに追加
  - [ ] `pop()`: BLPOP でブロッキング取得
- [ ] namespace 対応
  - [ ] Redis キーに namespace を prefix
- [ ] 接続管理
  - [ ] ConnectionManager の利用
  - [ ] 再接続ロジック
- [ ] テスト作成
  - [ ] Redis との統合テスト（testcontainers 推奨）

**学習ポイント**:
- Redis の基本操作（RPUSH/BLPOP）
- 非同期 Redis クライアントの使い方
- Testcontainers での統合テスト

**参考**: 要件ドキュメント 15章「DeliveryQueue」、18.2節「Ports 最小契約」

---

### PR-10: WorkerLoop（pop→claim→handle→decide→complete） ⏳ 未着手

**目的**: DeliveryQueue から task_id を取って実行する

- [ ] WorkerLoop の実装
  - [ ] `DeliveryQueue::pop()` で task_id 取得
  - [ ] `TaskStore::claim()` で lease 発行 + TaskEnvelope 取得
  - [ ] PayloadCodec で deserialize
  - [ ] Handler 実行 → Outcome
  - [ ] Decider 実行 → Decision
  - [ ] `TaskStore::complete()` で状態更新・履歴記録・依存解放・outbox生成
- [ ] エラーハンドリング
  - [ ] claim 失敗（他の worker が取った）→ 次へ
  - [ ] deserialize 失敗 → repair フロー（後のPR）
  - [ ] handler エラー → Outcome::failure
- [ ] ループロジック
  - [ ] 無限ループで pop を繰り返す
  - [ ] Graceful shutdown 対応
- [ ] テスト作成（モック/統合テスト）

**学習ポイント**:
- Worker パターン（pull model）
- Claim/Lease による並行制御
- 状態遷移とトランザクション境界

**参考**: 要件ドキュメント 3.2節「app/worker_loop.rs」、13章「状態遷移」

---

### PR-11: LeaseReaperLoop（期限切れ回収→再評価→outbox） ⏳ 未着手

**目的**: lease が期限切れになったタスクを回収して再配送

- [ ] LeaseReaperLoop の実装
  - [ ] `TaskStore::reap_expired_leases()` で期限切れを取得
  - [ ] running → pending/ready へ遷移
  - [ ] ready になったら outbox に dispatch_task を追加
- [ ] ループロジック
  - [ ] 定期的にポーリング（例: 10秒）
  - [ ] バッチ処理
- [ ] Graceful shutdown 対応
- [ ] テスト作成（モックでの動作確認）

**学習ポイント**:
- Lease/visibility timeout による at-least-once 保証
- 定期実行タスクの設計

**参考**: 要件ドキュメント 7章「実行保証」、3.2節「app/reaper_loop.rs」

---

### PR-12: ArtifactStore（MinIO/Local）+ artifactsメタ + TTL/GC ⏳ 未着手

**目的**: 巨大データを Blob に逃がし、TTL で自動削除

- [ ] ArtifactStore trait の定義
  - [ ] `async fn put(&self, ns: &str, bytes: Bytes, content_type: Option<&str>, ttl: Option<Duration>) -> Result<ArtifactHandle, ArtifactError>`
  - [ ] `async fn get(&self, ns: &str, artifact: ArtifactId) -> Result<Bytes, ArtifactError>`
  - [ ] `async fn delete(&self, ns: &str, artifact: ArtifactId) -> Result<(), ArtifactError>`
- [ ] weaver-blob クレート作成
  - [ ] MinIOArtifactStore 実装（object_store クレート推奨）
  - [ ] LocalArtifactStore 実装（ファイルシステム）
- [ ] PG artifacts テーブルとの連携
  - [ ] put() 時に PG にメタ情報記録（key, sha256, size, expires_at）
  - [ ] TTL 設定（expires_at）
- [ ] GC ループの実装
  - [ ] 定期的に expires_at < now の artifact を削除
  - [ ] PG の deleted_at を更新 + Blob 削除
- [ ] テスト作成
  - [ ] put/get/delete の動作確認
  - [ ] TTL/GC の動作確認

**学習ポイント**:
- Object storage の抽象化（S3/MinIO 互換）
- TTL による自動クリーンアップ
- PG メタ情報と Blob の整合性管理

**参考**: 要件ドキュメント 10章「Payload / Artifact / TTL」、18.3節「ArtifactStore」

---

### PR-13: Decode失敗→repairタスク→payload更新→再実行 ⏳ 未着手

**目的**: payload の decode 失敗を repair タスクで復旧

- [ ] RepairHintGenerator trait の定義
  - [ ] `async fn hint(&self, input: RepairHintInput) -> Result<RepairHint, RepairError>`
  - [ ] NoopRepairHintGenerator 実装（v2最小）
- [ ] 内部タスク: `weaver.internal.repair_payload.v1`
  - [ ] Handler 実装（RepairPayloadHandler）
  - [ ] hint に基づいて新しい artifact 生成（v2はダミーでOK）
- [ ] Worker での decode 失敗ハンドリング
  - [ ] Outcome::blocked with repair hint
  - [ ] Decider が repair タスク生成を提案
  - [ ] repair_count をインクリメント
  - [ ] max_repairs 超過で blocked/failed
- [ ] repair 成功フロー
  - [ ] `TaskStore::update_payload()` で payload 更新
  - [ ] `evaluate_readiness()` で ready 化
  - [ ] outbox に dispatch_task 追加
- [ ] テスト作成
  - [ ] repair フロー全体のテスト
  - [ ] max_repairs 超過のテスト

**学習ポイント**:
- エラー復旧の自動化
- 無限ループ防止（max_repairs）
- 内部タスクによるメタ処理

**参考**: 要件ドキュメント 11章「Decode失敗の復旧」

---

### PR-14: docker compose（全部入り）+ 統合テストシナリオ ⏳ 未着手

**目的**: ローカルで全システムを起動し、E2E で動作確認

- [ ] docker-compose.yml 作成
  - [ ] postgres（migrations 自動実行）
  - [ ] redis
  - [ ] minio（または local volume）
  - [ ] weaver-worker（複数インスタンス可）
  - [ ] outbox-publisher
  - [ ] lease-reaper
  - [ ] gc-loop
  - [ ] （任意）observability（jaeger など）
- [ ] Dockerfile 作成
  - [ ] weaver-cli のマルチステージビルド
  - [ ] 各ループを起動する entrypoint
- [ ] 統合テストシナリオ作成
  - [ ] 簡単なタスク実行（成功）
  - [ ] リトライフロー（失敗→成功）
  - [ ] タスク分解（parent→children）
  - [ ] 依存関係（DAG）
  - [ ] repair フロー
  - [ ] lease expiry 回収
- [ ] README の更新
  - [ ] 起動手順（`docker compose up`）
  - [ ] 動作確認手順
- [ ] DoD チェックリストの確認

**学習ポイント**:
- Docker Compose による複数サービスの管理
- マイグレーション自動実行
- E2E テストによる全体動作確認

**参考**: 要件ドキュメント 16章「Docker Compose」、19章「v2 DoD」

---

## 📚 参考資料

- **v2 要件**: `dev/docs/requirements/2026_01_03_weaver_requirements.md`（最新・正式）
- **v1 要件**: `dev/docs/requirements/2025_12_27_weaver_requirements.md`（参考）
- **アーキテクチャ**: `CLAUDE.md`
- **ADR**: `dev/docs/adr/`
- **v1 タスク**: `dev/learning/tasks_v1/tasks.md`
- **日々の実装記録**: `dev/learning/learning_YYYY_MM_DD.md`

---

## 📝 v2 学習の進め方

### 推奨順序

1. Week 1 を順番に実装（PR-1 → PR-6）
2. Week 2 を順番に実装（PR-7 → PR-14）
3. 各 PR で学習記録を `learning_YYYY_MM_DD.md` に残す
4. PR-14 完了後、v2 DoD を確認

### 学習のポイント

- **v1 との違いを意識**: 単一プロセス → 分散システムへの移行
- **Ports & Adapters パターン**: trait による抽象化と差し替え
- **トランザクション境界**: 状態遷移と outbox の整合性
- **非同期処理**: sqlx, redis, tokio の使いこなし
- **テスト戦略**: 単体 → 統合 → E2E の段階的テスト

### 質問・相談

実装中に不明点があれば、いつでも Claude Code に質問してください：
- 「この trait 設計で良いか？」
- 「トランザクション境界はどこに置くべき？」
- 「テストケースは何を書くべき？」

---

## 🎓 v1 で学んだこと（振り返り）

v1 実装を通じて学んだ主要な概念：

1. **所有権とライフタイム**: Arc/Mutex による共有状態管理
2. **非同期処理**: Tokio での async/await、spawn、timeout
3. **関数型パターン**: 純粋な Decider、副作用の分離
4. **Lock 最小化**: ADR-0003（never await while holding locks）
5. **テスト駆動**: 単体 → 統合 → E2E の段階的テスト

これらの知識を v2 でさらに深めていきます！
