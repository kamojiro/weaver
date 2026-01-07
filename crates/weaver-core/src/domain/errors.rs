//! Errors - エラー型と分類
//!
//! # 実装予定
//! - v2: ErrorKind の定義（運用分類）

/// ErrorKind は実行エラーの分類
///
/// # 分類（予定）
/// - Transient: 一時的なエラー（リトライ推奨）
/// - Permanent: 恒久的なエラー（リトライ無意味）
/// - Infrastructure: インフラエラー（PG/Redis/Blob の障害）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Transient,
    Permanent,
    Infrastructure,
}

/// WeaverError はドメインエラー
#[derive(Debug)]
pub struct WeaverError {
    kind: ErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl WeaverError {
    pub fn new(message: String) -> Self {
        Self {
            kind: ErrorKind::Transient,
            message,
            source: None,
        }
    }
}

impl std::fmt::Display for WeaverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WeaverError (kind: {:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for WeaverError {}
