// femtoClaw — 보안 모듈 내보내기
// [v0.1.0] 암복호화, Path Jailing, 블랙리스트 가드를 통합 관리하는 보안 계층.

pub mod crypto;

// [v0.1.0] Step 5에서 활성화 예정:
// pub mod jail;    // Path Jailing (디렉토리 탈출 차단)
// pub mod guard;   // 블랙리스트 커맨드 필터링
