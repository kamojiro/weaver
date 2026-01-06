//! PayloadCodec - artifact と Task の相互変換
//!
//! # 実装予定
//! - **PR-3**: PayloadCodec の実装

use super::task::Task;

/// PayloadCodec は artifact を T にデシリアライズ
///
/// # デシリアライズフロー
/// 1. ArtifactStore から bytes を取得
/// 2. serde_json で T にデシリアライズ
/// 3. 失敗時は repair フロー（PR-13）
pub struct PayloadCodec {
    // TODO(PR-3): フィールド定義
}

impl PayloadCodec {
    // TODO(PR-3): メソッド実装
    // - async fn decode<T: Task>(&self, artifact: ArtifactRef) -> Result<T, CodecError>
    // - async fn encode<T: Task>(&self, task: &T) -> Result<ArtifactRef, CodecError>
}
