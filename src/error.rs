// femtoClaw — 에러 타입 정의
// [v0.1.0] 프로젝트 전역에서 사용하는 통합 에러 타입.
// Result<T> 단축 타입을 제공하여 모든 모듈에서 일관된 에러 핸들링을 강제한다.

use thiserror::Error;

/// femtoClaw 전역 에러 타입.
/// 각 모듈별 에러를 variant로 포함하여 단일 에러 체인을 형성한다.
#[derive(Error, Debug)]
pub enum FemtoError {
    // --- 샌드박스 관련 ---
    /// 홈 디렉토리를 찾을 수 없는 경우 (dirs 크레이트 실패)
    #[error("홈 디렉토리를 찾을 수 없습니다")]
    HomeDirectoryNotFound,

    /// 샌드박스 디렉토리 생성 실패
    #[error("샌드박스 디렉토리 생성 실패: {0}")]
    SandboxCreation(#[source] std::io::Error),

    /// 프로세스 락 획득 실패 (이미 다른 인스턴스가 실행 중)
    #[error("femtoClaw가 이미 실행 중입니다 (PID: {pid})")]
    AlreadyRunning { pid: u32 },

    /// 락 파일 I/O 오류
    #[error("락 파일 처리 실패: {0}")]
    LockFileError(#[source] std::io::Error),

    // --- 암호화 관련 ---
    /// 마스터 패스워드로부터 키 파생 실패 (Argon2)
    #[error("암호화 키 파생 실패")]
    KeyDerivation,

    /// 데이터 암호화 실패 (ChaCha20Poly1305)
    #[error("데이터 암호화 실패")]
    Encryption,

    /// 데이터 복호화 실패 (비밀번호 오류 또는 데이터 손상)
    #[error("복호화 실패: 비밀번호가 올바르지 않거나 데이터가 손상되었습니다")]
    Decryption,

    // --- 설정 파일 관련 ---
    /// config.enc 파일 읽기/쓰기 실패
    #[error("설정 파일 I/O 오류: {0}")]
    ConfigIo(#[source] std::io::Error),

    /// config.enc 파일 형식이 올바르지 않음 (매직넘버 불일치 등)
    #[error("설정 파일 형식이 올바르지 않습니다")]
    InvalidConfigFormat,

    /// JSON 직렬화/역직렬화 실패
    #[error("설정 직렬화 오류: {0}")]
    Serialization(#[source] serde_json::Error),
}

/// femtoClaw 전역 Result 단축 타입.
/// 모든 함수에서 `Result<T>` 대신 `FemtoResult<T>`를 사용한다.
pub type FemtoResult<T> = std::result::Result<T, FemtoError>;
