# Lock 最小化パターン（ADR-0003 準拠）

## 問題

Lock（Mutex）を長時間保持すると、他の並行タスクがブロックされ、システムのスループットと応答性が低下します。

**アンチパターン:**
```rust
async fn operation(&self, data: Vec<Item>) -> Result<Vec<Id>, Error> {
    let mut state = self.state.lock().await;  // Lock 取得

    // Lock を持ったまま長時間の処理
    for item in &data {
        let id = state.allocate_id();
        let envelope = create_envelope(id, item);  // CPU 負荷の高い処理
        let record = create_record(envelope);      // CPU 負荷の高い処理
        state.records.insert(id, record);
    }

    Ok(ids)  // ここまで Lock を保持
}
```

## 解決策：3フェーズパターン

Lock が必要な操作と純粋な計算を分離する：

```rust
async fn operation(&self, data: Vec<Item>) -> Result<Vec<Id>, Error> {
    // フェーズ1: Lock を取得し、必要な情報だけを取得
    let (metadata, ids) = {
        let mut state = self.state.lock().await;

        // 親のメタデータを取得
        let metadata = extract_metadata(&state)?;

        // ID を事前に全て割り当て（Lock 内で行う必要がある）
        let ids: Vec<Id> = (0..data.len())
            .map(|_| state.allocate_id())
            .collect();

        (metadata, ids)
    }; // ここで Lock が解放される

    // フェーズ2: 純粋な計算（Lock 不要）
    let records: Vec<(Id, Record)> = data
        .into_iter()
        .zip(ids.iter())
        .map(|(item, &id)| {
            let envelope = create_envelope(id, item);
            let record = create_record(envelope, metadata);
            (id, record)
        })
        .collect();

    // フェーズ3: Lock を再取得し、一括更新
    {
        let mut state = self.state.lock().await;

        for (id, record) in records {
            state.records.insert(id, record);
            state.ready.push_back(id);
        }
    } // ここで Lock が解放される

    Ok(ids)
}
```

## 思考プロセス

### Step 1: 問題を認識する
- 「Lock を持ったまま長時間の処理をしている」
- 「他の並行タスクがブロックされている」

### Step 2: 依存関係を分析する
「何が Lock を必要とし、何が不要か？」を問う

**Lock が必要:**
- 共有状態の読み取り（親のメタデータ）
- 連番 ID の割り当て（順序性が重要）
- 共有状態の更新（records, ready キュー）

**Lock が不要:**
- Envelope の作成（純粋関数）
- Record の作成（純粋関数）
- データ変換（map, filter など）

### Step 3: フェーズに分割する

```
フェーズ1: Lock 必須の操作
  ├─ 共有状態からメタデータを読み取る
  └─ ID を割り当てる（連番、アトミックである必要がある）

フェーズ2: Lock 不要の操作
  ├─ Envelope を作成（純粋）
  └─ Record を作成（純粋）

フェーズ3: Lock 必須の操作
  ├─ Records を挿入（共有状態）
  └─ Ready キューを更新（共有状態）
```

### Step 4: Rust の機能を活用する
- **スコープ `{}`**: Lock を早期に解放
- **`map().collect()`**: Immutable な変換
- **`zip()`**: 複数のイテレータを組み合わせ
- **所有権のある データ**: Lock のスコープ外にデータを移動

## 核心原則

**「共有状態へのアクセス」と「純粋な計算」を分離する**

このパターンが適用できる場面：
- データベース接続 + 重い計算
- ファイルハンドル + 処理
- 任意の排他リソース + CPU 処理

## ADR-0003 準拠

このパターンは以下を保証します：
1. **Lock を持ったまま `.await` しない**（デッドロックを防ぐ）
2. **Lock の保持時間を最小化**（並行性を最大化）
3. **Lock の早期解放**（スコープで明示的に）

## 実際の例

`weaver-core/src/queue/memory.rs` より：

```rust
async fn add_child_tasks(&self, child_specs: Vec<TaskSpec>)
    -> Result<Vec<TaskId>, WeaverError>
{
    // フェーズ1: 親の情報取得 + ID 割り当て
    let (parent_job_id, max_attempts, task_ids) = {
        let mut state = self.queue.lock().await;
        let parent = state.records.get(&self.task_id)?;
        let parent_job_id = parent.job_id?;
        let max_attempts = parent.max_attempts;
        let task_ids: Vec<TaskId> = (0..child_specs.len())
            .map(|_| state.allocate_task_id())
            .collect();
        (parent_job_id, max_attempts, task_ids)
    }; // Lock 解放

    // フェーズ2: Record を作成（Lock なし）
    let task_records: Vec<(TaskId, TaskRecord)> = child_specs
        .into_iter()
        .zip(task_ids.iter())
        .map(|(spec, &task_id)| {
            let envelope = TaskEnvelope::new(task_id, spec.task_type, spec.payload);
            let record = TaskRecord::new_child(envelope, max_attempts, parent_job_id, self.task_id);
            (task_id, record)
        })
        .collect();

    // フェーズ3: 一括挿入（Lock を再取得）
    {
        let mut state = self.queue.lock().await;
        for (task_id, record) in task_records {
            state.records.insert(task_id, record);
            state.ready.push_back(task_id);
        }
    } // Lock 解放

    Ok(task_ids)
}
```

## メリット

1. **Lock 競合の削減**: 必要な時だけ Lock を保持
2. **並行性の向上**: フェーズ2 の間、他のタスクが進行できる
3. **コード構造の明確化**: フェーズ分割により依存関係が明示的
4. **推論しやすい**: 純粋な操作と状態を持つ操作が分離されている
5. **デッドロック防止**: Lock を持ったまま `.await` しない

## Before/After の比較

### Before: Lock を長時間保持

```rust
async fn add_child_tasks(&self, child_specs: Vec<TaskSpec>) -> Result<Vec<TaskId>, WeaverError> {
    let mut state = self.queue.lock().await;  // Lock 取得

    let (parent_job_id, max_attempts) = {
        let parent = state.records.get(&self.task_id)?;
        (parent.job_id?, parent.max_attempts)
    };

    let mut task_ids = Vec::new();
    for spec in &child_specs {
        let task_id = state.allocate_task_id();
        let envelope = TaskEnvelope::new(task_id, spec.task_type.clone(), spec.payload.clone());
        let record = TaskRecord::new_child(envelope, max_attempts, parent_job_id, self.task_id);
        state.records.insert(task_id, record);
        state.ready.push_back(task_id);
        task_ids.push(task_id);
    }  // ← ここまで Lock を保持（ループ全体）

    Ok(task_ids)
}
```

**問題点:**
- Lock を保持したままループ処理
- TaskEnvelope, TaskRecord の作成が Lock 内
- 他のタスクがブロックされる時間が長い

### After: Lock 最小化

```rust
async fn add_child_tasks(&self, child_specs: Vec<TaskSpec>) -> Result<Vec<TaskId>, WeaverError> {
    // フェーズ1: Lock を取って必要な情報だけ取得
    let (parent_job_id, max_attempts, task_ids) = {
        let mut state = self.queue.lock().await;
        let parent = state.records.get(&self.task_id)?;
        let parent_job_id = parent.job_id?;
        let max_attempts = parent.max_attempts;
        let task_ids: Vec<TaskId> = (0..child_specs.len())
            .map(|_| state.allocate_task_id())
            .collect();
        (parent_job_id, max_attempts, task_ids)
    }; // ← Lock 即座に解放

    // フェーズ2: Lock 外で Record を作成
    let task_records: Vec<(TaskId, TaskRecord)> = child_specs
        .into_iter()
        .zip(task_ids.iter())
        .map(|(spec, &task_id)| {
            let envelope = TaskEnvelope::new(task_id, spec.task_type, spec.payload);
            let record = TaskRecord::new_child(envelope, max_attempts, parent_job_id, self.task_id);
            (task_id, record)
        })
        .collect();  // ← この間、Lock は保持していない

    // フェーズ3: Lock を再取得して一括更新
    {
        let mut state = self.queue.lock().await;
        for (task_id, record) in task_records {
            state.records.insert(task_id, record);
            state.ready.push_back(task_id);
        }
    } // ← Lock 即座に解放

    Ok(task_ids)
}
```

**改善点:**
- Lock の保持時間が大幅に短縮
- フェーズ2（重い処理）の間、他のタスクが進行可能
- コードの意図が明確（フェーズ分割）

## 関連項目

- ADR-0003: Lock を持ったまま `.await` しない
- 関数型プログラミング: 純粋関数と副作用の分離
- データベースのベストプラクティス: 短いトランザクション

## 応用：async 関数を呼ぶ場合

Lock を持ったまま async 関数を呼ぶ必要がある場合：

```rust
Decision::Decompose { child_tasks, reason } => {
    // フェーズ1: 必要な情報を取得
    let task_id = self.task_id;

    // フェーズ2: Lock を明示的に drop
    drop(state);

    // フェーズ3: async 処理（Lock なし）
    let child_ids = self.add_child_tasks(child_tasks).await?;

    // フェーズ4: Lock を再取得
    let mut state = self.queue.lock().await;

    // フェーズ5: 状態更新
    if let Some(record) = state.records.get_mut(&task_id) {
        record.state = TaskState::Decomposed;
    }

    // DecisionRecord を記録
    let decision_record = DecisionRecord::new(/*...*/);
    state.decisions.push(decision_record);

    false  // terminal state
}
```

**ポイント:**
- `drop(state)` で Lock を明示的に解放
- async 処理は Lock 外で実行
- Lock を再取得して状態を更新
