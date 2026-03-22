// femtoClaw — ZSTD 압축/해제 유틸리티
// [v0.1.0] Step 3: 라즈베리 파이 32GB SD카드 수명 연장을 위해
// 대화 로그, 행동 내역 등 텍스트 데이터를 ZSTD로 압축하여 DB에 저장한다.
// 일반 텍스트 대비 5~10배 용량 절약 효과.

/// ZSTD 압축 레벨 (1~22, 기본 3이면 속도/압축률 균형 우수)
const COMPRESSION_LEVEL: i32 = 3;

/// 텍스트 데이터를 ZSTD로 압축한다.
/// 실패 시 원본 데이터를 그대로 반환하여 데이터 유실을 방지한다.
pub fn compress_data(data: &[u8]) -> Vec<u8> {
    zstd::encode_all(data, COMPRESSION_LEVEL).unwrap_or_else(|_| data.to_vec())
}

/// ZSTD 압축 데이터를 해제한다.
pub fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, String> {
    zstd::decode_all(compressed)
        .map_err(|e| format!("ZSTD 해제 실패: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        // 원본 텍스트 → 압축 → 해제 → 원본과 일치해야 함
        let original = b"Hello, femtoClaw! This is a test message for ZSTD compression.";
        let compressed = compress_data(original);

        // 압축 결과는 원본보다 작거나 같아야 함 (짧은 입력은 오히려 커질 수 있음)
        assert!(!compressed.is_empty());

        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_compress_large_repetitive_data() {
        // 반복 데이터는 높은 압축률을 보여야 함
        let original: Vec<u8> = "에이전트 응답 로그 ".repeat(1000).into_bytes();
        let compressed = compress_data(&original);

        // 반복 데이터는 최소 5배 이상 압축되어야 함
        assert!(compressed.len() < original.len() / 5,
            "압축률 부족: 원본 {} → 압축 {} ({}배)",
            original.len(), compressed.len(), original.len() / compressed.len()
        );

        let decompressed = decompress_data(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decompress_invalid_data() {
        // 유효하지 않은 압축 데이터 → 에러 반환
        let invalid = b"this is not zstd compressed data";
        let result = decompress_data(invalid);
        assert!(result.is_err());
    }
}
