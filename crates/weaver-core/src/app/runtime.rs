//! Runtime - 型付き Task API の表面
//!
//! 既存の runtime.rs を統合する予定

/// Runtime は型付き Task API を提供
///
/// # 使用例（予定）
/// ```ignore
/// runtime.enqueue_typed(MyTask { ... }).await?;
/// ```
pub struct Runtime {
    // TODO(Phase 2): 既存 runtime.rs から移行
}

impl Runtime {
    // TODO(Phase 2): メソッド実装
}
