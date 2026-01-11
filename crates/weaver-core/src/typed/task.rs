//! Task trait - 型付き Task の定義
//!
//! # 学習ポイント
//! - Associated Constants (`const TYPE`)
//! - Trait bounds の組み合わせ (Serialize + DeserializeOwned + Send + Sync + 'static)

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Task は task_type と型を対応付ける
///
/// # 使用例
/// ```ignore
/// #[derive(Serialize, Deserialize)]
/// struct MyTask {
///     message: String,
/// }
///
/// impl Task for MyTask {
///     const TYPE: &'static str = "my_namespace.my_task.v1";
/// }
/// ```
///
/// # Trait Bounds
/// - `Serialize`: artifact への保存のため
/// - `DeserializeOwned`: artifact からの復元のため（'static に対応）
/// - `Send + Sync`: 複数スレッドから安全に使えるため
/// - `'static`: Arc に格納できるため（参照を持たない）
pub trait Task: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// task_type の定義
    ///
    /// # 命名規約
    /// - `{namespace}.{domain}.{action}.v{major}`
    /// - 例: `acme.billing.charge.v1`
    const TYPE: &'static str;
}

// 一時的にテスト用の Task 型をいくつか定義します。
// 将来的には、これらは別のテストモジュールに移動する予定です。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTask {
    pub value: i32,
}

impl Task for TestTask {
    const TYPE: &'static str = "test.task.create.v1";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnotherTestTask{
    pub name: String,
    pub family: HashMap<String, String>,
}

impl Task for AnotherTestTask {
    const TYPE: &'static str = "test.task.another.v1";
}
