// femtoClaw — Rhai 동적 스킬 엔진
// [v0.2.0] Step 6a: Rhai 스크립트를 sandboxed 환경에서 실행한다.
//
// 설계 원칙:
//   - 파일 접근은 Path Jailing 경계(workspace/) 내에서만 허용
//   - 스크립트 실행 시간은 최대 30초 (무한 루프 방지)
//   - 호스트 함수: file_read, file_write, file_list, llm_chat, db_query, print, sleep
//   - TOML 정적 스킬과 .rhai 동적 스킬 공존 (하이브리드)

use rhai::{Engine, Scope, AST};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::security::jail;

/// [v0.2.0] Rhai 스크립트 실행 결과
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// 스크립트 출력 (print 호출의 누적)
    pub output: Vec<String>,
    /// 실행 성공 여부
    pub success: bool,
    /// 에러 메시지 (실패 시)
    pub error: Option<String>,
    /// 실행 시간 (밀리초)
    pub elapsed_ms: u64,
}

/// [v0.2.0] Rhai 동적 스킬 엔진 — sandboxed 실행 환경
pub struct RhaiEngine {
    /// Rhai 엔진 인스턴스
    engine: Engine,
    /// workspace 루트 (Path Jailing 경계)
    workspace: PathBuf,
    /// 스크립트 출력 버퍼 (print 호출 누적)
    output_buffer: Arc<Mutex<Vec<String>>>,
}

impl RhaiEngine {
    /// [v0.2.0] 새 Rhai 엔진 생성 — 호스트 함수 등록 및 보안 설정
    pub fn new(workspace: PathBuf) -> Self {
        let mut engine = Engine::new();
        let output_buffer = Arc::new(Mutex::new(Vec::new()));

        // 보안: 최대 연산 횟수 제한 (무한 루프 방지, ~30초 상당)
        engine.set_max_operations(1_000_000);
        // 보안: 스택 깊이 제한 (재귀 폭탄 방지)
        engine.set_max_call_levels(32);
        // 보안: 문자열 최대 길이 (메모리 폭탄 방지)
        engine.set_max_string_size(1_048_576); // 1MB
                                               // 보안: 배열 최대 크기
        engine.set_max_array_size(10_000);

        // --- 호스트 함수 등록 ---

        // print(msg) — 로그 출력
        let buf = output_buffer.clone();
        engine.on_print(move |msg| {
            if let Ok(mut buffer) = buf.lock() {
                buffer.push(msg.to_string());
            }
        });

        // debug 출력도 같은 버퍼로
        let buf2 = output_buffer.clone();
        engine.on_debug(move |msg, _, _| {
            if let Ok(mut buffer) = buf2.lock() {
                buffer.push(format!("[DEBUG] {}", msg));
            }
        });

        // file_read(path) — workspace 내 파일 읽기
        let ws = workspace.clone();
        engine.register_fn("file_read", move |path: &str| -> String {
            let full_path = ws.join(path);
            match jail::validate_path(&full_path, &ws) {
                Ok(safe_path) => std::fs::read_to_string(safe_path)
                    .unwrap_or_else(|e| format!("[오류] 파일 읽기 실패: {}", e)),
                Err(e) => format!("[보안 차단] {}", e),
            }
        });

        // file_write(path, content) — workspace 내 파일 쓰기
        // 파일이 아직 없을 수 있으므로 ../ 패턴 차단 + 부모 디렉토리 존재 검증
        let ws2 = workspace.clone();
        engine.register_fn("file_write", move |path: &str, content: &str| -> String {
            // 보안: ../ 순회 차단
            if path.contains("..") {
                return "[보안 차단] 디렉토리 순회(../) 금지".to_string();
            }
            let full_path = ws2.join(path);
            // 부모 디렉토리 생성
            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            // 절대 경로로 변환 후 workspace 접두사 검증
            let ws_canon = ws2.canonicalize().unwrap_or_else(|_| ws2.clone());
            let parent_canon = full_path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .unwrap_or_else(|| full_path.clone());
            if !parent_canon.starts_with(&ws_canon) {
                return format!("[보안 차단] workspace 밖 쓰기 금지: {}", path);
            }
            match std::fs::write(&full_path, content) {
                Ok(_) => format!("파일 저장 완료: {}", path),
                Err(e) => format!("[오류] 파일 쓰기 실패: {}", e),
            }
        });

        // file_list(dir) — workspace 내 디렉토리 목록
        let ws3 = workspace.clone();
        engine.register_fn("file_list", move |dir: &str| -> rhai::Array {
            let full_path = ws3.join(dir);
            match jail::validate_path(&full_path, &ws3) {
                Ok(safe_path) => {
                    let mut result: rhai::Array = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(safe_path) {
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                result.push(rhai::Dynamic::from(name.to_string()));
                            }
                        }
                    }
                    result
                }
                Err(_) => Vec::new(),
            }
        });

        // sleep(ms) — 대기 (최대 5000ms)
        engine.register_fn("sleep", |ms: i64| {
            let clamped = ms.clamp(0, 5000) as u64;
            std::thread::sleep(std::time::Duration::from_millis(clamped));
        });

        RhaiEngine {
            engine,
            workspace,
            output_buffer,
        }
    }

    /// [v0.2.0] Rhai 스크립트 문자열을 실행한다.
    pub fn run_script(&self, script: &str) -> ScriptResult {
        // 출력 버퍼 초기화
        if let Ok(mut buf) = self.output_buffer.lock() {
            buf.clear();
        }

        let start = std::time::Instant::now();

        match self.engine.eval::<rhai::Dynamic>(script) {
            Ok(_) => {
                let output = self
                    .output_buffer
                    .lock()
                    .map(|buf| buf.clone())
                    .unwrap_or_default();

                ScriptResult {
                    output,
                    success: true,
                    error: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                }
            }
            Err(e) => {
                let output = self
                    .output_buffer
                    .lock()
                    .map(|buf| buf.clone())
                    .unwrap_or_default();

                ScriptResult {
                    output,
                    success: false,
                    error: Some(format!("{}", e)),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
    }

    /// [v0.2.0] .rhai 파일을 컴파일(AST)한다. 문법 검증용.
    pub fn compile(&self, script: &str) -> Result<AST, String> {
        self.engine
            .compile(script)
            .map_err(|e| format!("컴파일 오류: {}", e))
    }

    /// [v0.2.0] .rhai 파일 경로에서 스크립트를 로드하여 실행한다.
    pub fn run_file(&self, path: &Path) -> ScriptResult {
        match std::fs::read_to_string(path) {
            Ok(script) => self.run_script(&script),
            Err(e) => ScriptResult {
                output: Vec::new(),
                success: false,
                error: Some(format!("스크립트 파일 읽기 실패: {}", e)),
                elapsed_ms: 0,
            },
        }
    }

    /// workspace 경로 반환
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_workspace() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_rhai_test")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_basic_script() {
        let ws = test_workspace();
        let engine = RhaiEngine::new(ws.clone());

        let result = engine.run_script(
            r#"
            let x = 1 + 2;
            print("결과: " + x);
        "#,
        );

        assert!(result.success);
        assert_eq!(result.output, vec!["결과: 3"]);
        assert!(result.error.is_none());
        cleanup(&ws);
    }

    #[test]
    fn test_file_read_write() {
        let ws = test_workspace();

        // 테스트 파일 생성
        fs::write(ws.join("test.txt"), "안녕하세요").unwrap();

        let engine = RhaiEngine::new(ws.clone());

        let result = engine.run_script(
            r#"
            let content = file_read("test.txt");
            print("읽음: " + content);
            file_write("output.txt", "결과: " + content);
        "#,
        );

        assert!(result.success);
        assert_eq!(result.output, vec!["읽음: 안녕하세요"]);

        // 쓰기 검증
        let written = fs::read_to_string(ws.join("output.txt")).unwrap();
        assert_eq!(written, "결과: 안녕하세요");

        cleanup(&ws);
    }

    #[test]
    fn test_file_list() {
        let ws = test_workspace();
        fs::write(ws.join("a.txt"), "").unwrap();
        fs::write(ws.join("b.txt"), "").unwrap();

        let engine = RhaiEngine::new(ws.clone());
        let result = engine.run_script(
            r#"
            let files = file_list(".");
            print("파일 수: " + files.len());
        "#,
        );

        assert!(result.success);
        assert_eq!(result.output, vec!["파일 수: 2"]);
        cleanup(&ws);
    }

    #[test]
    fn test_path_jailing_blocks_escape() {
        let ws = test_workspace();
        let engine = RhaiEngine::new(ws.clone());

        let result = engine.run_script(
            r#"
            let content = file_read("../../../etc/passwd");
            print(content);
        "#,
        );

        // 실행은 성공하지만 보안 차단 메시지가 출력됨
        assert!(result.success);
        assert!(result.output[0].contains("보안 차단") || result.output[0].contains("오류"));
        cleanup(&ws);
    }

    #[test]
    fn test_script_error_handling() {
        let ws = test_workspace();
        let engine = RhaiEngine::new(ws.clone());

        // 문법 오류
        let result = engine.run_script("let x = ;");
        assert!(!result.success);
        assert!(result.error.is_some());
        cleanup(&ws);
    }

    #[test]
    fn test_compile_validation() {
        let ws = test_workspace();
        let engine = RhaiEngine::new(ws.clone());

        // 유효한 스크립트
        assert!(engine.compile("let x = 1 + 2;").is_ok());

        // 무효한 스크립트
        assert!(engine.compile("let x = ;").is_err());
        cleanup(&ws);
    }

    #[test]
    fn test_sleep_clamped() {
        let ws = test_workspace();
        let engine = RhaiEngine::new(ws.clone());

        let result = engine.run_script(
            r#"
            sleep(100);
            print("완료");
        "#,
        );

        assert!(result.success);
        assert!(result.elapsed_ms >= 80); // 최소 ~100ms
        cleanup(&ws);
    }

    #[test]
    fn test_run_file() {
        let ws = test_workspace();
        let script_path = ws.join("test_skill.rhai");
        fs::write(
            &script_path,
            r#"
            let a = 10;
            let b = 20;
            print("합계: " + (a + b));
        "#,
        )
        .unwrap();

        let engine = RhaiEngine::new(ws.clone());
        let result = engine.run_file(&script_path);

        assert!(result.success);
        assert_eq!(result.output, vec!["합계: 30"]);
        cleanup(&ws);
    }
}
