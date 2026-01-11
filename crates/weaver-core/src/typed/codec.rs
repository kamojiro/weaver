//! PayloadCodec - serde_json::Value と Task の相互変換
//!
//! # 学習ポイント
//! - serde_json::Value の扱い
//! - Generic functions での型変換
//! - エラーハンドリング（Result composition）

use super::task::Task;

/// PayloadCodec は Task と serde_json::Value 間の変換を担当
///
/// # v2 の設計
/// - artifact からの bytes 取得は別レイヤー（ArtifactStore）
/// - PayloadCodec は純粋に serde_json::Value ⟷ Task の変換のみ
///
/// # デシリアライズフロー（将来）
/// 1. ArtifactStore から bytes を取得
/// 2. serde_json::from_slice() で Value に変換
/// 3. PayloadCodec::decode() で Task に変換
/// 4. 失敗時は repair フロー（PR-13）
pub struct PayloadCodec;

/// CodecError は encode/decode のエラー
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("Serialization failed: {0}")]
    SerializeFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializeFailed(String),
}

impl PayloadCodec {

    pub fn encode<T: Task>(task: &T) -> Result<serde_json::Value, CodecError>{
        let value = serde_json::to_value(task)
            .map_err(|e| CodecError::SerializeFailed(e.to_string()))?;
        Ok(value)
    }

    pub fn decode<T: Task>(payload: serde_json::Value) -> Result<T, CodecError>{
        let task = serde_json::from_value::<T>(payload)
            .map_err(|e| CodecError::DeserializeFailed(e.to_string()))?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed::task::TestTask;
    use serde_json::json;
    #[test]
    fn test_encode_decode_roundtrip() {
        let original_task = TestTask { value: 42 };
        let encoded = PayloadCodec::encode(&original_task).expect("Encoding failed");
        let decoded: TestTask = PayloadCodec::decode(encoded).expect("Decoding failed");
        assert_eq!(original_task.value, decoded.value);
    }   
    #[test]
    fn test_decode_invalid_payload() {
        let invalid_payload = json!({"invalid_field": "not an integer"});
        let result: Result<TestTask, CodecError> = PayloadCodec::decode(invalid_payload);
        match result {
            Err(CodecError::DeserializeFailed(_)) => (),
            _ => panic!("Expected DeserializeFailed error"),
        }
    }
}   