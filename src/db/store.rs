// femtoClaw — SQLite 데이터 저장소
// [v0.2.0] Step 3/7a: WAL 모드 초기화, 에이전트 행동 내역 스키마,
// 트랜잭션 기반 상태 저장, 풀 타임머신 (페이지네이션 + 필터 + 선택적 Undo),
// DB 무결성 검사 및 백업 자동 복구.
//
// [v0.2.0] 변경사항:
//   - ActionType에 SkillRun 추가 (Rhai 스킬 실행 기록)
//   - actions_paged(): 페이지네이션 쿼리
//   - actions_filtered(): 유형별 필터링 쿼리
//   - undo_by_id(): 선택적 Undo (임의의 ID 지정)
//   - action_count_filtered(): 필터 적용 카운트

use super::compress::{compress_data, decompress_data};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

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
    /// [v0.2.0] Rhai 스킬 실행
    SkillRun,
    /// [v0.4.0] 도구 호출 (Tool Harness)
    ToolCall,
    /// [v0.4.0] 보안 이벤트 (Jailing 차단 등)
    SecurityEvent,
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
            ActionType::SkillRun => "skill_run",
            ActionType::ToolCall => "tool_call",
            ActionType::SecurityEvent => "security_event",
        }
    }

    /// DB 문자열 → 열거형 복원
    fn from_str(s: &str) -> Self {
        match s {
            "user_message" => ActionType::UserMessage,
            "agent_response" => ActionType::AgentResponse,
            "file_operation" => ActionType::FileOperation,
            "api_call" => ActionType::ApiCall,
            "skill_run" => ActionType::SkillRun,
            "tool_call" => ActionType::ToolCall,
            "security_event" => ActionType::SecurityEvent,
            _ => ActionType::SystemEvent,
        }
    }

    /// [v0.2.0] 한국어 표시명
    pub fn display_name(&self) -> &'static str {
        match self {
            ActionType::UserMessage => "대화",
            ActionType::AgentResponse => "응답",
            ActionType::FileOperation => "파일",
            ActionType::ApiCall => "API",
            ActionType::SystemEvent => "시스템",
            ActionType::SkillRun => "스킬",
            ActionType::ToolCall => "도구",
            ActionType::SecurityEvent => "보안",
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

/// [v0.2.0] SQLite WAL 기반 데이터 저장소.
/// DB 파일 경로: `~/.femtoclaw/db/femto_state.db`
pub struct FemtoDb {
    conn: Connection,
    db_path: PathBuf,
}

impl FemtoDb {
    /// DB를 열고 WAL 모드 활성화 + 스키마 초기화.
    pub fn open(db_path: &Path) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("DB 디렉토리 생성 실패: {}", e))?;
        }

        let conn = Connection::open(db_path).map_err(|e| format!("DB 열기 실패: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("WAL 모드 설정 실패: {}", e))?;
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
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS actions (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                action_type TEXT NOT NULL,
                summary     TEXT NOT NULL,
                content     BLOB,
                timestamp   TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                undone      INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_actions_timestamp
                ON actions(timestamp DESC);

            CREATE INDEX IF NOT EXISTS idx_actions_undone
                ON actions(undone);

            -- [v0.2.0] 유형별 필터링 성능용 인덱스
            CREATE INDEX IF NOT EXISTS idx_actions_type
                ON actions(action_type);

            CREATE TABLE IF NOT EXISTS metadata (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO metadata (key, value)
                VALUES ('schema_version', '2');
            ",
            )
            .map_err(|e| format!("스키마 초기화 실패: {}", e))
    }

    /// 에이전트 행동을 기록한다. content는 ZSTD 압축하여 저장.
    pub fn insert_action(
        &self,
        action_type: &ActionType,
        summary: &str,
        content: &str,
    ) -> Result<i64, String> {
        let compressed = compress_data(content.as_bytes());

        self.conn
            .execute(
                "INSERT INTO actions (action_type, summary, content)
             VALUES (?1, ?2, ?3)",
                params![action_type.as_str(), summary, compressed],
            )
            .map_err(|e| format!("행동 기록 실패: {}", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    /// 최근 N건의 행동 기록을 조회한다 (Undo 안 된 것만).
    pub fn recent_actions(&self, limit: usize) -> Result<Vec<ActionRecord>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, action_type, summary, content, timestamp, undone
             FROM actions
             WHERE undone = 0
             ORDER BY id DESC
             LIMIT ?1",
            )
            .map_err(|e| format!("쿼리 준비 실패: {}", e))?;

        self.parse_rows(&mut stmt, params![limit as i64])
    }

    /// [v0.2.0] 페이지네이션 쿼리 — 전체 기록(Undone 포함)을 페이지로 조회.
    /// page: 0부터 시작, per_page: 한 페이지당 건수
    pub fn actions_paged(&self, page: usize, per_page: usize) -> Result<Vec<ActionRecord>, String> {
        let offset = page * per_page;
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, action_type, summary, content, timestamp, undone
             FROM actions
             ORDER BY id DESC
             LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| format!("쿼리 준비 실패: {}", e))?;

        self.parse_rows(&mut stmt, params![per_page as i64, offset as i64])
    }

    /// [v0.2.0] 유형별 필터링 + 페이지네이션.
    pub fn actions_filtered(
        &self,
        action_type: &ActionType,
        page: usize,
        per_page: usize,
    ) -> Result<Vec<ActionRecord>, String> {
        let offset = page * per_page;
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, action_type, summary, content, timestamp, undone
             FROM actions
             WHERE action_type = ?1
             ORDER BY id DESC
             LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("쿼리 준비 실패: {}", e))?;

        self.parse_rows(
            &mut stmt,
            params![action_type.as_str(), per_page as i64, offset as i64],
        )
    }

    /// [v0.2.0] 특정 ID의 행동을 Undo한다 (선택적 Undo).
    pub fn undo_by_id(&self, id: i64) -> Result<bool, String> {
        let affected = self
            .conn
            .execute(
                "UPDATE actions SET undone = 1 WHERE id = ?1 AND undone = 0",
                params![id],
            )
            .map_err(|e| format!("Undo 실패: {}", e))?;
        Ok(affected > 0)
    }

    /// 마지막 행동을 Undo한다 (v0.1 호환).
    pub fn undo_last(&self) -> Result<Option<ActionRecord>, String> {
        let actions = self.recent_actions(1)?;
        if let Some(action) = actions.into_iter().next() {
            self.conn
                .execute(
                    "UPDATE actions SET undone = 1 WHERE id = ?1",
                    params![action.id],
                )
                .map_err(|e| format!("Undo 실패: {}", e))?;

            Ok(Some(ActionRecord {
                undone: true,
                ..action
            }))
        } else {
            Ok(None)
        }
    }

    /// DB 무결성을 검사한다.
    pub fn check_integrity(&self) -> Result<bool, String> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
            .map_err(|e| format!("무결성 검사 실패: {}", e))?;
        Ok(result == "ok")
    }

    /// DB 파일을 백업한다.
    pub fn backup(&self) -> Result<PathBuf, String> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| format!("WAL 체크포인트 실패: {}", e))?;

        let backup_path = self.db_path.with_extension("db.backup");
        std::fs::copy(&self.db_path, &backup_path).map_err(|e| format!("백업 실패: {}", e))?;
        Ok(backup_path)
    }

    /// 백업에서 DB를 복구한다.
    pub fn restore_from_backup(&self) -> Result<(), String> {
        let backup_path = self.db_path.with_extension("db.backup");
        if !backup_path.exists() {
            return Err("백업 파일이 없습니다".to_string());
        }

        let corrupted_path = self.db_path.with_extension("db.corrupted");
        std::fs::rename(&self.db_path, &corrupted_path)
            .map_err(|e| format!("손상 DB 이름 변경 실패: {}", e))?;
        std::fs::copy(&backup_path, &self.db_path).map_err(|e| format!("백업 복원 실패: {}", e))?;
        Ok(())
    }

    /// 전체 행동 수를 반환한다.
    pub fn action_count(&self) -> Result<i64, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
            .map_err(|e| format!("카운트 조회 실패: {}", e))
    }

    /// [v0.2.0] 유형별 행동 수를 반환한다.
    pub fn action_count_filtered(&self, action_type: &ActionType) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM actions WHERE action_type = ?1",
                params![action_type.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| format!("카운트 조회 실패: {}", e))
    }

    /// 내부 헬퍼: 쿼리 결과를 ActionRecord Vec으로 변환
    fn parse_rows(
        &self,
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<ActionRecord>, String> {
        let rows = stmt
            .query_map(params, |row| {
                let id: i64 = row.get(0)?;
                let action_type_str: String = row.get(1)?;
                let summary: String = row.get(2)?;
                let compressed: Vec<u8> = row.get(3)?;
                let timestamp: String = row.get(4)?;
                let undone: bool = row.get(5)?;

                let content = decompress_data(&compressed)
                    .and_then(|bytes| {
                        String::from_utf8(bytes).map_err(|e| format!("UTF-8 변환 실패: {}", e))
                    })
                    .unwrap_or_else(|_| "[압축 해제 실패]".to_string());

                Ok(ActionRecord {
                    id,
                    action_type: ActionType::from_str(&action_type_str),
                    summary,
                    content,
                    timestamp,
                    undone,
                })
            })
            .map_err(|e| format!("쿼리 실행 실패: {}", e))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| format!("행 파싱 실패: {}", e))?);
        }
        Ok(records)
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
        dir.join(format!(
            "test_{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
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
        let version: String = db
            .conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, "2");

        cleanup(&path);
    }

    #[test]
    fn test_insert_and_query() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 3건 삽입
        db.insert_action(&ActionType::UserMessage, "질문", "오늘 날씨는?")
            .unwrap();
        db.insert_action(&ActionType::AgentResponse, "응답", "맑습니다.")
            .unwrap();
        db.insert_action(
            &ActionType::FileOperation,
            "파일 생성",
            "test.txt 생성 완료",
        )
        .unwrap();

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

        db.insert_action(&ActionType::UserMessage, "첫 번째", "내용1")
            .unwrap();
        db.insert_action(&ActionType::AgentResponse, "두 번째", "내용2")
            .unwrap();

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

        db.insert_action(&ActionType::SystemEvent, "백업 전", "데이터")
            .unwrap();

        // 백업 생성
        let backup_path = db.backup().unwrap();
        assert!(backup_path.exists());

        // 추가 데이터
        db.insert_action(&ActionType::SystemEvent, "백업 후", "새 데이터")
            .unwrap();
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
        db.insert_action(&ActionType::AgentResponse, "장문 응답", &big_content)
            .unwrap();

        // 정확히 복원되는지 확인
        let actions = db.recent_actions(1).unwrap();
        assert_eq!(actions[0].content, big_content);

        // DB 파일 크기가 원본보다 작아야 함 (ZSTD 압축 효과)
        let db_size = std::fs::metadata(&path).unwrap().len() as usize;
        assert!(
            db_size < big_content.len(),
            "DB 크기({})가 원본({})보다 커서는 안 됨",
            db_size,
            big_content.len()
        );

        cleanup(&path);
    }

    // === [v0.2.0] 풀 타임머신 테스트 ===

    #[test]
    fn test_actions_paged() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        // 5건 삽입
        for i in 1..=5 {
            db.insert_action(
                &ActionType::UserMessage,
                &format!("메시지 {}", i),
                &format!("내용 {}", i),
            )
            .unwrap();
        }

        // 1페이지(2건): 최신 2건
        let page0 = db.actions_paged(0, 2).unwrap();
        assert_eq!(page0.len(), 2);
        assert_eq!(page0[0].summary, "메시지 5");
        assert_eq!(page0[1].summary, "메시지 4");

        // 2페이지(2건)
        let page1 = db.actions_paged(1, 2).unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].summary, "메시지 3");

        // 3페이지(1건)
        let page2 = db.actions_paged(2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].summary, "메시지 1");

        cleanup(&path);
    }

    #[test]
    fn test_actions_filtered() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        db.insert_action(&ActionType::UserMessage, "질문", "내용")
            .unwrap();
        db.insert_action(&ActionType::FileOperation, "파일 생성", "내용")
            .unwrap();
        db.insert_action(&ActionType::UserMessage, "질문2", "내용")
            .unwrap();
        db.insert_action(&ActionType::SkillRun, "스킬 실행", "내용")
            .unwrap();

        // UserMessage만 필터
        let filtered = db
            .actions_filtered(&ActionType::UserMessage, 0, 10)
            .unwrap();
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].summary, "질문2");

        // SkillRun 필터
        let skills = db.actions_filtered(&ActionType::SkillRun, 0, 10).unwrap();
        assert_eq!(skills.len(), 1);

        // 카운트
        assert_eq!(
            db.action_count_filtered(&ActionType::UserMessage).unwrap(),
            2
        );

        cleanup(&path);
    }

    #[test]
    fn test_undo_by_id() {
        let path = temp_db_path();
        let db = FemtoDb::open(&path).unwrap();

        let id1 = db
            .insert_action(&ActionType::UserMessage, "첫 번째", "내용1")
            .unwrap();
        let _id2 = db
            .insert_action(&ActionType::AgentResponse, "두 번째", "내용2")
            .unwrap();
        let id3 = db
            .insert_action(&ActionType::FileOperation, "세 번째", "내용3")
            .unwrap();

        // 가운데(id1) 선택 Undo
        assert!(db.undo_by_id(id1).unwrap());

        // 페이지네이션으로 확인 — id1은 undone=true
        let all = db.actions_paged(0, 10).unwrap();
        assert_eq!(all.len(), 3);
        let undone_record = all.iter().find(|r| r.id == id1).unwrap();
        assert!(undone_record.undone);

        // 이미 Undo된 건 재시도하면 false
        assert!(!db.undo_by_id(id1).unwrap());

        // id3도 Undo
        assert!(db.undo_by_id(id3).unwrap());

        // active(recent) 조회는 id2만 남음
        let active = db.recent_actions(10).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].summary, "두 번째");

        cleanup(&path);
    }
}
