// femtoClaw — 데이터베이스 엔진 모듈
// [v0.1.0] Step 3: SQLite WAL + ZSTD 압축 기반 에이전트 상태 관리
//
// 구조:
//   FemtoDb        — DB 연결 관리, WAL 초기화, 무결성 검사
//   ActionRecord   — 에이전트 행동 기록 (대화, 파일 조작, API 호출 등)
//   UndoManager    — 최근 5건 표시 + 마지막 동작 Undo

pub mod compress;
pub mod store;

pub use compress::{compress_data, decompress_data};
pub use store::FemtoDb;
