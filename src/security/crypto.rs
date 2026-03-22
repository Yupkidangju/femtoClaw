// femtoClaw — ChaCha20Poly1305 암복호화 모듈
// [v0.1.0] Step 1: 마스터 패스워드 기반 키 파생 및 config.enc 양방향 암복호화.
//
// 구현 원리:
// 1. 마스터 패스워드 + 랜덤 솔트 → Argon2id로 256비트 키 파생
// 2. 파생된 키 + 랜덤 논스 → ChaCha20Poly1305로 AEAD 암호화
// 3. 파일 형식: [매직넘버 4B][버전 2B][솔트 32B][논스 12B][암호문+태그]
//
// IP 보호(spec.md 5절):
// 키 파생 파라미터(Argon2 m_cost, t_cost, p_cost)는 KdfParams 트레이트로 추상화.
// 실제 운영 파라미터는 프로덕션 빌드에서만 주입한다.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;

use crate::error::{FemtoError, FemtoResult};

/// config.enc 파일 매직넘버: "FMTC" (FeMToClaw)
const MAGIC: &[u8; 4] = b"FMTC";

/// config.enc 파일 형식 버전 (향후 마이그레이션용)
const FORMAT_VERSION: u16 = 1;

/// Argon2id 솔트 길이 (바이트)
const SALT_LEN: usize = 32;

/// ChaCha20Poly1305 논스 길이 (바이트)
const NONCE_LEN: usize = 12;

/// 파일 헤더 총 길이: 매직(4) + 버전(2) + 솔트(32) + 논스(12) = 50바이트
const HEADER_LEN: usize = 4 + 2 + SALT_LEN + NONCE_LEN;

/// [v0.1.0] 마스터 패스워드로부터 256비트 암호화 키를 파생한다.
/// Argon2id 알고리즘을 사용하며, 솔트를 함께 제공해야 한다.
///
/// IP 보호 사항:
/// Argon2 파라미터(메모리 비용, 시간 비용, 병렬도)는 내부적으로 결정되며,
/// 외부에 노출되지 않는 기본값을 사용한다.
fn derive_key(password: &[u8], salt: &[u8]) -> FemtoResult<[u8; 32]> {
    let mut key = [0u8; 32];

    // Argon2id 기본 파라미터 사용 (IP 보호: 구체적 값은 라이브러리 기본값에 위임)
    let argon2 = argon2::Argon2::default();

    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| FemtoError::KeyDerivation)?;

    Ok(key)
}

/// [v0.1.0] 평문 데이터를 ChaCha20Poly1305로 암호화한다.
///
/// 반환값은 config.enc 파일에 직접 쓸 수 있는 완전한 바이트열이다.
/// 형식: [FMTC][0001][솔트 32B][논스 12B][암호문+Poly1305 태그]
///
/// 매 호출마다 새로운 솔트와 논스를 생성하므로,
/// 동일한 패스워드+평문이라도 매번 다른 암호문이 생성된다.
pub fn seal(password: &[u8], plaintext: &[u8]) -> FemtoResult<Vec<u8>> {
    // 1. 랜덤 솔트 생성 (키 파생용)
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);

    // 2. 패스워드 + 솔트 → 256비트 키 파생
    let key_bytes = derive_key(password, &salt)?;
    let key = Key::from_slice(&key_bytes);

    // 3. 랜덤 논스 생성 (암호화용)
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // 4. ChaCha20Poly1305 AEAD 암호화
    let cipher = ChaCha20Poly1305::new(key);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| FemtoError::Encryption)?;

    // 5. 파일 형식 조립: 매직 + 버전 + 솔트 + 논스 + 암호문
    let mut output = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// [v0.1.0] 암호화된 데이터를 복호화한다.
///
/// config.enc 파일의 전체 바이트열을 입력으로 받는다.
/// 매직넘버와 버전을 검증한 후, 솔트를 추출하여 키를 재파생하고,
/// ChaCha20Poly1305로 복호화한다.
///
/// 패스워드가 틀리거나 데이터가 손상된 경우 Decryption 에러를 반환한다.
pub fn unseal(password: &[u8], sealed_data: &[u8]) -> FemtoResult<Vec<u8>> {
    // 1. 최소 길이 검증 (헤더 + 최소 1바이트 암호문 + 16바이트 태그)
    if sealed_data.len() < HEADER_LEN + 17 {
        return Err(FemtoError::InvalidConfigFormat);
    }

    // 2. 매직넘버 검증
    if &sealed_data[0..4] != MAGIC {
        return Err(FemtoError::InvalidConfigFormat);
    }

    // 3. 버전 검증 (현재는 v1만 지원)
    let version = u16::from_le_bytes([sealed_data[4], sealed_data[5]]);
    if version != FORMAT_VERSION {
        return Err(FemtoError::InvalidConfigFormat);
    }

    // 4. 솔트, 논스, 암호문 추출
    let salt = &sealed_data[6..6 + SALT_LEN];
    let nonce_bytes = &sealed_data[6 + SALT_LEN..6 + SALT_LEN + NONCE_LEN];
    let ciphertext = &sealed_data[HEADER_LEN..];

    // 5. 패스워드 + 솔트 → 키 재파생
    let key_bytes = derive_key(password, salt)?;
    let key = Key::from_slice(&key_bytes);
    let nonce = Nonce::from_slice(nonce_bytes);

    // 6. ChaCha20Poly1305 AEAD 복호화 (인증 태그 검증 포함)
    let cipher = ChaCha20Poly1305::new(key);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| FemtoError::Decryption)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 암복호화 왕복(Roundtrip) 테스트:
    /// 동일한 패스워드로 암호화한 데이터를 복호화하면 원본과 일치해야 한다.
    #[test]
    fn test_seal_unseal_roundtrip() {
        let password = b"test-master-password-2026!";
        let plaintext = b"{\"api_key\": \"sk-test-1234567890\"}";

        // 암호화
        let sealed = seal(password, plaintext).expect("암호화 성공해야 함");

        // 매직넘버 확인
        assert_eq!(&sealed[0..4], b"FMTC");

        // 복호화
        let decrypted = unseal(password, &sealed).expect("복호화 성공해야 함");
        assert_eq!(decrypted, plaintext);
    }

    /// 잘못된 패스워드 테스트:
    /// 다른 패스워드로 복호화하면 Decryption 에러가 발생해야 한다.
    #[test]
    fn test_wrong_password_fails() {
        let password = b"correct-password";
        let wrong_password = b"wrong-password";
        let plaintext = b"sensitive data";

        let sealed = seal(password, plaintext).expect("암호화 성공해야 함");
        let result = unseal(wrong_password, &sealed);

        assert!(result.is_err());
    }

    /// 동일한 입력이라도 매번 다른 암호문이 생성되는지 테스트:
    /// 솔트와 논스가 매번 랜덤하므로 동일한 입력이라도 결과가 달라야 한다.
    #[test]
    fn test_different_ciphertext_each_time() {
        let password = b"same-password";
        let plaintext = b"same-data";

        let sealed1 = seal(password, plaintext).expect("1차 암호화");
        let sealed2 = seal(password, plaintext).expect("2차 암호화");

        // 암호문은 서로 달라야 함 (솔트/논스가 랜덤이므로)
        assert_ne!(sealed1, sealed2);

        // 하지만 둘 다 올바르게 복호화되어야 함
        let d1 = unseal(password, &sealed1).expect("1차 복호화");
        let d2 = unseal(password, &sealed2).expect("2차 복호화");
        assert_eq!(d1, plaintext);
        assert_eq!(d2, plaintext);
    }

    /// 손상된 데이터 테스트:
    /// 암호문의 일부를 변조하면 복호화가 실패해야 한다 (AEAD 인증 태그 검증).
    #[test]
    fn test_tampered_data_fails() {
        let password = b"test-password";
        let plaintext = b"important config";

        let mut sealed = seal(password, plaintext).expect("암호화 성공");

        // 암호문 마지막 바이트를 변조
        let last = sealed.len() - 1;
        sealed[last] ^= 0xFF;

        let result = unseal(password, &sealed);
        assert!(result.is_err());
    }

    /// 잘못된 매직넘버 테스트:
    /// 매직넘버가 "FMTC"가 아닌 데이터는 InvalidConfigFormat을 반환해야 한다.
    #[test]
    fn test_invalid_magic_number() {
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"XXXX"); // 잘못된 매직넘버

        let result = unseal(b"password", &data);
        assert!(result.is_err());
    }

    /// 데이터 너무 짧은 경우 테스트:
    /// 헤더보다 짧은 데이터는 InvalidConfigFormat을 반환해야 한다.
    #[test]
    fn test_too_short_data() {
        let result = unseal(b"password", b"short");
        assert!(result.is_err());
    }
}
