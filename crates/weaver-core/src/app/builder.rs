//! AppBuilder - アプリケーションの構築とワイヤリング
//!
//! # 学習ポイント
//! - Builder パターンの実装
//! - 起動時検証（Fail-fast 設計）
//! - 開発体験の改善（明確なエラーメッセージ）

use crate::typed::{Handler, RegistryError, Task, TypedRegistry};

/// AppBuilder はアプリケーションを構築
///
/// # 使用例
/// ```ignore
/// let app = AppBuilder::new()
///     .register::<MyTask>(MyTaskHandler)
///     .expect_tasks(&["my_namespace.my_task.v1"])
///     .build()?;
/// ```
///
/// # Fail-fast 設計
/// - expect_tasks() で期待される task_type を登録
/// - build() 時に「期待集合 ⊆ 登録済み集合」をチェック
/// - 不足があれば BuildError を返す
pub struct AppBuilder {
    registry: TypedRegistry,
    expected_tasks: Option<Vec<String>>,
}

/// BuildError はアプリケーション構築時のエラー
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("Missing task types: {0:?}. These tasks were expected but not registered.")]
    MissingTaskTypes(Vec<String>),
}

impl AppBuilder {
    /// 新しい AppBuilder を作成
    pub fn new() -> Self {
        Self {
            registry: TypedRegistry::new(),
            expected_tasks: None,
        }
    }

    /// Handler を登録
    ///
    /// # Example
    /// ```ignore
    /// builder.register::<MyTask>(MyTaskHandler)?;
    /// ```
    pub fn register<T: Task, H: Handler<T> + 'static>(
        mut self,
        handler: H,
    ) -> Result<Self, RegistryError> {
        self.registry.register::<T, H>(handler)?;
        Ok(self)
    }

    /// 期待される task_type のリストを設定
    ///
    /// # Example
    /// ```ignore
    /// builder.expect_tasks(&["my_namespace.my_task.v1"])?;
    /// ```
    pub fn expect_tasks(mut self, task_types: &[&str]) -> Self {
        let mut expected_tasks = Vec::new();
        for &task_type in task_types {
            expected_tasks.push(task_type.to_string());
        }
        self.expected_tasks = Some(expected_tasks);
        self
    }

    /// AppBuilder を構築して App を生成
    ///
    /// # 検証
    /// - expect_tasks() で設定された task_type が全て登録されているかチェック
    /// - 不足があれば BuildError::MissingTaskTypes を返す
    ///
    /// # Example
    /// ```ignore
    /// let app = builder.build()?;
    /// ```
    pub fn build(self) -> Result<App, BuildError> {
        if let Some(expected_tasks) = &self.expected_tasks {
            let registered_types = self.registry.registered_types();
            let missing_tasks: Vec<String> = expected_tasks
                .iter()
                .filter(|x| !registered_types.contains(x))
                .cloned()
                .collect::<Vec<String>>();
            if !missing_tasks.is_empty() {
                return Err(BuildError::MissingTaskTypes(missing_tasks));
            }
        }
        Ok(App {
            registry: self.registry,
        })
    }
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// App はアプリケーションのランタイム
///
/// # v2 最小版
/// - TypedRegistry のみを保持（起動時検証のデモ用）
/// - 将来: TaskStore, DeliveryQueue, ArtifactStore などを追加
pub struct App {
    pub registry: TypedRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::handler::{TestTaskHandler};
    use crate::typed::task::{AnotherTestTask, TestTask};

    #[test]
    fn test_build_success() {
        let app = AppBuilder::new()
            .register::<TestTask, _>(TestTaskHandler {})
            .unwrap()
            .expect_tasks(&[TestTask::TYPE])
            .build();
        assert!(app.is_ok());
    }

    #[test]
    fn test_build_missing_task_types() {
        let app = AppBuilder::new()
            .register::<TestTask, _>(TestTaskHandler {})
            .unwrap()
            .expect_tasks(&[TestTask::TYPE, AnotherTestTask::TYPE])
            .build();
        assert!(matches!(
            app,
            Err(BuildError::MissingTaskTypes(missing)) if missing == vec![AnotherTestTask::TYPE.to_string()]
        ));
    }

    #[test]
    fn test_build_no_expect_tasks() {
        let app = AppBuilder::new()
            .register::<TestTask, _>(TestTaskHandler {})
            .unwrap()
            .build();
        assert!(app.is_ok());
    }
}
