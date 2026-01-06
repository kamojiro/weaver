//! ArtifactStore port - Blob ストレージ（MinIO/S3/Local）
//!
//! ArtifactStore は巨大データ（payload, context）を保存します。
//!
//! # 実装予定
//! - **PR-12**: MinIOArtifactStore, LocalArtifactStore

/// ArtifactStore は巨大データを Blob に保存
///
/// # 設計原則
/// - TTL（expires_at）をサポート
/// - PG の artifacts テーブルにメタ情報を記録
/// - GC ループで期限切れを削除
pub trait ArtifactStore {
    // TODO(PR-12): メソッド定義
    // - async fn put(&self, ns: &str, bytes: Bytes, content_type: Option<&str>, ttl: Option<Duration>) -> Result<ArtifactHandle, ArtifactError>
    // - async fn get(&self, ns: &str, artifact: ArtifactId) -> Result<Bytes, ArtifactError>
    // - async fn delete(&self, ns: &str, artifact: ArtifactId) -> Result<(), ArtifactError>
}
