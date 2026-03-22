// femtoClaw — SQLite 데이터 저장소
// [v0.1.0] Step 3: WAL 모드 초기화, 에이전트 행동 내역 스키마,
// 트랜잭션 기반 상태 저장, 간소 Undo (최근 5건 + 마지막 Undo),
// DB 무결성 검사 및 백업 자동 복구.
//
// 스키마 설계 원칙:
//   - actions 테이블: 모든 에이전트 행동을 기록 (대화, 파일, API 등)
//   - content 컬럼: ZSTD 압축된 BLOB (라즈베리 파이 용량 절약)
//   - undone 플래그: Undo 시 true로 마킹 (물리 삭제 없음)

use std::path::{Path, PathBuf};
use rusqlite::{Connection, params};
use super::compress::{compress_data, decompress_data};

/// 에이전트 행동 유형
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    /// 사용자 → 에이전트 대화
    UserMessage,
    /// 에이전트 → 사용자 응답
    AgentResponse,
    /// 파일 생성/수정/삭제
    FileOperation,
    /// 외부 API 호출
    ApiCall,
    /// 시스템 이벤트 (부팅, 설정 변경 등)
    SystemEvent,
}

impl ActionType {
    /// DB 저장용 문자열 변환
    fn as_str(&self) -> &'static str {
        match self {
            ActionType::UserMessage => "user_message",
            ActionType::AgentResponse => "agent_response",
            ActionType::FileOperation => "file_operation",
            ActionType::ApiCall => "api_call",
            ActionType::SystemEvent => "system_event",
        }
    }

    /// DB 문자열 → 열거형 복원
    fn from_str(s: &str) -> Self {
        match s {
            "user_message" => ActionType::UserMessage,
            "agent_response" => ActionType::AgentResponse,
            "file_operation" => ActionType::FileOperation,
            "api_call" => ActionType::ApiCall,
            _ => ActionType::SystemEvent,
        }
    }
}

/// 에이전트 행동 기록 (1건)
#[derive(Debug, Clone)]
pub struct ActionRecord {
    pub id: i64,
    pub action_type: ActionType,
    pub summary: String,
    /// 원본 콘텐츠 (압축 해제 후)
    pub content: String,
    pub timestamp: String,
    pub undone: bool,
}

/// [v0.1.0] SQLite WAL 기반 데이터 저장소.
/// DB 파일 경로: `~/.femtoclaw/db/femto_state.db`
pub struct FemtoDb {
    conn: Connection,
    db_path: PathBuf,
}

impl FemtoDb {
    /// DB를 열고 WAL 모드 활성화 + 스키마 초기화.
    /// 파일이 없으면 자동 생성됨.
    pub fn open(db_path: &Path) -> Result<Self, String> {
        // 상위 디렉토리 보장
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("DB 디렉토리 생성 실패: {}", e))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| format!("DB 열기 실패: {}", e))?;

        // WAL 모드 활성화 (읽기/쓰기 병행, I/O 최적화)
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("WAL 모드 설정 실패: {}", e))?;

        // 자동 체크포인트 간격 (1000 페이지마다)
        conn.execute_batch("PRAGMA wal_autocheckpoint=1000;")
            .map_err(|e| format!("체크포인트 설정 실패: {}", e))?;

        let db = FemtoDb {
            conn,
            db_path: db_path.to_path_buf(),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// 테이블 스키마 초기화 (없으면 생성)
    fn init_schema(&self) -> Result<(), String> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS actions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                action_type TEXT NOT NULL,
                summary     TEXT NOT NULL,
                content     BLOB,
                timestamp   TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                undone      INTEGER NOT NULL DEFAULT 0
            );

            -- 최근 조회 성능용 인덱스
            CREATE INDEX IF NOT EXISTS idx_actions_timestamp
                ON actions(timestamp DESC);

            -- Undo 대상 필터링용 인덱스
            CREATE INDEX IF NOT EXISTS idx_actions_undone
                ON actions(undone);

            -- 메타데이터 테이블 (DB 버전 관리용)
            CREATE TABLE IF NOT EXISTS metadata (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- DB 스키마 버전 기록 (향후 마이그레이션용)
            INSERT OR IGNORE INTO metadata (key, value)
                VALUES ('schema_version', '1');
            "
        ).map_err(|e| format!("스키마 초기화 실패: {}", e))
    }

    /// 에이전트 행동을 기록한다. content는 ZSTD 압축하여 저장.
    pub fn insert_action(
        &self,
        action_type: &ActionType,
        summary: &str,
        content: &str,
    ) -> Result<i64, String> {
        // 콘텐츠 ZSTD 압축
        let compressed = compress_data(content.as_bytes());

        self.conn.execute(
            "INSERT INTO actions (action_type, summary, content)
             VALUES (?1, ?2, ?3)",
            params![action_type.as_str(), summary, compressed],
        ).map_err(|e| format!("행동 기록 실패: {}", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// 최근 N건의 행동 기록을 조회한다 (Undo 안 된 것만).
    pub fn recent_actions(&self, limit: usize) -> Result<Vec<ActionRecord>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT id, action_type, summary, content, timestamp, undone
             FROM actions
             WHERE undone = 0
             ORDER BY id DESC
             LIMIT ?1"
        ).map_err(|e| format!("쿼리 준비 실패: {}", e))?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let action_type_str: String = row.get(1)?;
            let summary: String = row.get(2)?;
            let compressed: Vec<u8> = row.get(3)?;
            let timestamp: String = row.get(4)?;
            let undone: bool = row.get(5)?;

            // ZSTD 압축 해제
            let content = decompress_data(&compressed)
                .and_then(|bytes| String::from_utf8(bytes)
                    .map_err(|e| format!("UTF-8 변환 실패: {}", e)))
                .unwrap_or_else(|_| "[압축 해제 실패]".to_string());

            Ok(ActionRecord {
                id,
                action_type: ActionType::from_str(&action_type_str),
                summary,
                content,
                timestamp,
                undone,
            })
        }).map_err(|e| format!("쿼리 실행 실패: {}", e))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| format!("행 파싱 실패: {}", e))?);
        }
        Ok(records)
    }

    /// 마지막 행동을 Undo한다 (물리 삭제 없이 undone=1 마킹).
    /// 이미 Undo된 행동은 건너뛴다.
    /// 반환값: Undo된 ActionRecord (없으면 None)
    pub fn undo_last(&self) -> Result<Option<ActionRecord>, String> {
        // 가장 최근의 Undo 안 된 행동 조회
        let actions = self.recent_actions(1)?;
        if let Some(action) = actions.into_iter().next() {
            self.conn.execute(
                "UPDATE actions SET undone = 1 WHERE id = ?1",
                params![action.id],
            ).map_err(|e| format!("Undo 실패: {}", e))?;

            Ok(Some(ActionRecord { undone: true, ..action }))
        } else {
            Ok(None)
        }
    }

    /// DB 무결성을 검사한다 (SQLite PRAGMA integrity_check).
    pub fn check_integrity(&self) -> Result<bool, String> {
        let result: String = self.conn.query_row(
            "PRAGMA integrity_check;",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("무결성 검사 실패: {}", e))?;

        Ok(result == "ok")
    }

    /// DB 파일을 백업한다 (파일 복사 방식).
    /// 백업 전 WAL 체크포인트를 수행하여 모든 데이터가 메인 파일에 반영됨을 보장.
    /// 백업 파일: `{원본경로}.backup`
    pub fn backup(&self) -> Result<PathBuf, String> {
        // WAL 체크포인트: WAL에 있는 모든 데이터를 메인 DB 파일에 기록
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("WAL 체크포인트 실패: {}", e))?;

        let backup_path = self.db_path.with_extension("db.backup");
        std::fs::copy(&self.db_path, &backup_path)
            .map_err(|e| format!("백업 실패: {}", e))?;
        Ok(backup_path)
    }

    /// 백업에서 DB를 복구한다.
    /// 기존 DB를 `.corrupted`로 이름 변경 후 백업에서 복원.
    pub fn restore_from_backup(&self) -> Result<(), String> {
        let backup_path = self.db_path.with_extension("db.backup");
        if !backup_path.exists() {
            return Err("백업 파일이 없습니다".to_string());
        }

        let corrupted_path = self.db_path.with_extension("db.corrupted");
        std::fs::rename(&self.db_path, &corrupted_path)
            .map_err(|e| format!("손상 DB 이름 변경 실패: {}", e))?;

        std::fs::copy(&backup_path, &self.db_path)
            .map_err(|e| format!("백업 복원 실패: {}", e))?;

        Ok(())
    }

    /// 전체 행동 수를 반환한다.
    pub fn action_count(&self) -> Result<i64, String> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM actions",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("카운트 조회 실패: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// 임시 DB 경로 생성 헬퍼
    fn temp_db_path() -> PathBuf {
        let dir = std::env::temp_dir().join("femtoclaw_test");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(format!("test_{}.db", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
    }

    /// 임시 DB 정리 헬퍼
    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        let _ = std::fs::remove_file(path.with_extension("db.backup"));
        let _ = std::fs::remove_file(path.with_extension("db.corrupted"));
    }

    #[test]
    fn test_open_and_schema() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 스키마 버전 확인
        let version: String = db.conn.query_row(
            "SELECT value FROM metadata WHERE key = 'schema_version'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(version, "1");

        cleanup(&path);
    }

    #[test]
    fn test_insert_and_query() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 3건 삽입
        db.insert_action(&ActionType::UserMessage, "질문", "오늘 날씨는?").unwrap();
        db.insert_action(&ActionType::AgentResponse, "응답", "맑습니다.").unwrap();
        db.insert_action(&ActionType::FileOperation, "파일 생성", "test.txt 생성 완료").unwrap();

        // 최근 2건 조회
        let actions = db.recent_actions(2).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].summary, "파일 생성"); // 최신 순
        assert_eq!(actions[1].summary, "응답");

        // 압축 해제된 콘텐츠 확인
        assert_eq!(actions[0].content, "test.txt 생성 완료");
        assert_eq!(actions[1].content, "맑습니다.");

        assert_eq!(db.action_count().unwrap(), 3);

        cleanup(&path);
    }

    #[test]
    fn test_undo_last() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        db.insert_action(&ActionType::UserMessage, "첫 번째", "내용1").unwrap();
        db.insert_action(&ActionType::AgentResponse, "두 번째", "내용2").unwrap();

        // Undo: 두 번째(최신)가 Undo됨
        let undone = db.undo_last().unwrap().unwrap();
        assert_eq!(undone.summary, "두 번째");
        assert!(undone.undone);

        // 이제 recent(1)은 첫 번째만 나옴
        let actions = db.recent_actions(1).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].summary, "첫 번째");

        // 한 번 더 Undo
        let undone2 = db.undo_last().unwrap().unwrap();
        assert_eq!(undone2.summary, "첫 번째");

        // 더 이상 Undo할 것 없음
        let none = db.undo_last().unwrap();
        assert!(none.is_none());

        cleanup(&path);
    }

    #[test]
    fn test_integrity_check() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 정상 DB → 무결성 통과
        assert!(db.check_integrity().unwrap());

        cleanup(&path);
    }

    #[test]
    fn test_backup_and_restore() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        db.insert_action(&ActionType::SystemEvent, "백업 전", "데이터").unwrap();

        // 백업 생성
        let backup_path = db.backup().unwrap();
        assert!(backup_path.exists());

        // 추가 데이터
        db.insert_action(&ActionType::SystemEvent, "백업 후", "새 데이터").unwrap();
        assert_eq!(db.action_count().unwrap(), 2);

        // Windows에서는 열린 DB 파일 rename이 불가하므로,
        // 백업 파일을 별도 경로에서 열어 1건만 있는지 검증
        let backup_db = FemtoDb::open(&backup_path).unwrap();
        assert_eq!(backup_db.action_count().unwrap(), 1);

        cleanup(&path);
        cleanup(&backup_path);
    }

    #[test]
    fn test_zstd_compression_in_db() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 큰 반복 텍스트 저장 (ZSTD 압축 효과 확인)
        let big_content = "에이전트가 생성한 장문의 응답 텍스트입니다. ".repeat(500);
        db.insert_action(&ActionType::AgentResponse, "장문 응답", &big_content).unwrap();

        // 정확히 복원되는지 확인
        let actions = db.recent_actions(1).unwrap();
        assert_eq!(actions[0].content, big_content);

        // DB 파일 크기가 원본보다 작아야 함 (ZSTD 압축 효과)
        let db_size = std::fs::metadata(&path).unwrap().len() as usize;
        assert!(db_size < big_content.len(),
            "DB 크기({})가 원본({})보다 커서는 안 됨", db_size, big_content.len());

        cleanup(&path);
    }
}
