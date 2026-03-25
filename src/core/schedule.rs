// femtoClaw — 내장 스케줄러 모듈
// [v0.8.0] schedule.toml 기반 예약 작업 엔진
//
// 설계:
//   - schedule.toml에서 크론 표현식 + 액션 이름을 파싱
//   - 60초 간격 타이머 루프에서 현재 시각과 크론 패턴 매칭
//   - 매칭 시 해당 액션 실행 (memory_cleanup, db_backup, daily_summary)
//
// 크론 형식: 분 시 일 월 요일 (5-필드, * = 와일드카드, */N = 간격)

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 스케줄 설정 전체 구조
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleConfig {
    /// 예약 작업 목록
    #[serde(default)]
    pub tasks: Vec<ScheduleTask>,
}

/// 개별 예약 작업 정의
#[derive(Debug, Deserialize, Clone)]
pub struct ScheduleTask {
    /// 작업 이름 (식별용)
    pub name: String,
    /// 크론 표현식 (분 시 일 월 요일)
    pub cron: String,
    /// 실행할 액션 이름
    pub action: String,
}

/// [v0.8.0] 5-필드 크론 패턴
#[derive(Debug, Clone)]
pub struct CronPattern {
    pub minute: CronField,
    pub hour: CronField,
    pub day: CronField,
    pub month: CronField,
    pub weekday: CronField,
}

/// 크론 필드 타입
#[derive(Debug, Clone)]
pub enum CronField {
    /// * — 모든 값 매칭
    Any,
    /// 특정 값 (예: 3)
    Exact(u32),
    /// 간격 (예: */6 → 0,6,12,18)
    Interval(u32),
}

impl CronField {
    /// 필드 문자열을 파싱한다
    fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s == "*" {
            return Ok(CronField::Any);
        }
        if let Some(interval) = s.strip_prefix("*/") {
            let n: u32 = interval
                .parse()
                .map_err(|_| format!("잘못된 간격: {}", s))?;
            if n == 0 {
                return Err("간격은 0보다 커야 합니다".to_string());
            }
            return Ok(CronField::Interval(n));
        }
        let n: u32 = s.parse().map_err(|_| format!("잘못된 크론 값: {}", s))?;
        Ok(CronField::Exact(n))
    }

    /// 현재 값이 이 필드와 매칭되는지 확인한다
    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Exact(v) => *v == value,
            CronField::Interval(n) => value % n == 0,
        }
    }
}

impl CronPattern {
    /// 크론 표현식 문자열을 파싱한다 (5-필드)
    pub fn parse(expr: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(format!(
                "크론 표현식은 5개 필드가 필요합니다 (받음: {})",
                parts.len()
            ));
        }

        Ok(CronPattern {
            minute: CronField::parse(parts[0])?,
            hour: CronField::parse(parts[1])?,
            day: CronField::parse(parts[2])?,
            month: CronField::parse(parts[3])?,
            weekday: CronField::parse(parts[4])?,
        })
    }

    /// 현재 시각이 이 크론 패턴과 매칭되는지 확인한다
    pub fn matches_now(&self) -> bool {
        let now = chrono::Local::now();
        let minute = now.format("%M").to_string().parse::<u32>().unwrap_or(0);
        let hour = now.format("%H").to_string().parse::<u32>().unwrap_or(0);
        let day = now.format("%d").to_string().parse::<u32>().unwrap_or(1);
        let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
        // chrono: 월=0(일), ..., 6(토) → 크론: 0(일), ..., 6(토)
        let weekday = now.format("%w").to_string().parse::<u32>().unwrap_or(0);

        self.minute.matches(minute)
            && self.hour.matches(hour)
            && self.day.matches(day)
            && self.month.matches(month)
            && self.weekday.matches(weekday)
    }
}

/// [v0.8.0] schedule.toml 로드
pub fn load_config(workspace: &Path) -> Option<ScheduleConfig> {
    let path = workspace.join("schedule.toml");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    toml::from_str(&content).ok()
}

/// [v0.8.0] 기본 schedule.toml 생성 (workspace에 없을 경우)
pub fn create_default_config(workspace: &Path) -> std::io::Result<PathBuf> {
    let path = workspace.join("schedule.toml");
    if path.exists() {
        return Ok(path);
    }

    let default = r#"# femtoClaw 예약 작업 설정
# 크론 형식: 분 시 일 월 요일 (* = 전체, */N = N간격)

[[tasks]]
name = "memory_cleanup"
cron = "0 3 * * *"         # 매일 새벽 3시
action = "memory_cleanup"

[[tasks]]
name = "db_backup"
cron = "0 */6 * * *"       # 6시간마다 정각
action = "db_backup"

[[tasks]]
name = "daily_summary"
cron = "0 22 * * *"        # 매일 밤 10시
action = "daily_summary"
"#;

    std::fs::write(&path, default)?;
    Ok(path)
}

/// [v0.8.0] 예약 액션 실행
/// action 문자열에 따라 해당 기능을 수행한다.
pub fn execute_action(action: &str, workspace: &Path, db_path: &Path) {
    match action {
        "memory_cleanup" => action_memory_cleanup(workspace),
        "db_backup" => action_db_backup(db_path),
        "daily_summary" => action_daily_summary(workspace),
        _ => {
            eprintln!("[스케줄러] 알 수 없는 액션: {}", action);
        }
    }
}

/// 액션: MEMORY.md FIFO 정리 (100줄 초과 시 오래된 항목 삭제)
fn action_memory_cleanup(workspace: &Path) {
    let memory_path = workspace.join("MEMORY.md");
    if !memory_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&memory_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();
    let header_count = lines.iter().take_while(|l| !l.starts_with("- [")).count();
    let data_count = lines.len() - header_count;

    if data_count > 100 {
        let trim = data_count - 100;
        let kept: Vec<&str> = lines[..header_count]
            .iter()
            .chain(lines[header_count + trim..].iter())
            .copied()
            .collect();
        let _ = std::fs::write(&memory_path, kept.join("\n") + "\n");
        eprintln!(
            "[스케줄러] memory_cleanup: {}줄 정리 ({}→100)",
            trim, data_count
        );
    }
}

/// 액션: SQLite DB 백업 (db/ 디렉토리에 날짜 복사본 생성)
fn action_db_backup(db_path: &Path) {
    let today = chrono::Local::now().format("%Y%m%d").to_string();
    let backup_name = format!("femto_state_backup_{}.db", today);

    if let Some(db_dir) = db_path.parent() {
        let backup_path = db_dir.join(backup_name);
        if backup_path.exists() {
            // 오늘 이미 백업됨
            return;
        }
        match std::fs::copy(db_path, &backup_path) {
            Ok(bytes) => {
                eprintln!(
                    "[스케줄러] db_backup: {} ({} bytes)",
                    backup_path.display(),
                    bytes
                );
            }
            Err(e) => {
                eprintln!("[스케줄러] db_backup 실패: {}", e);
            }
        }
    }
}

/// 액션: 일일 로그 마지막 5건 요약 → MEMORY.md 추가
fn action_daily_summary(workspace: &Path) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_path = workspace.join("memory").join(format!("{}.md", today));

    if !log_path.exists() {
        return;
    }

    let content = match std::fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // 대화 섹션 수 세기
    let conversation_count = content.matches("### ").count();
    if conversation_count == 0 {
        return;
    }

    // MEMORY.md에 요약 추가
    let memory_path = workspace.join("MEMORY.md");
    let summary = format!(
        "- [{}] 📊 일일 요약: {}건의 대화 기록\n",
        today, conversation_count
    );

    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&memory_path)
    {
        let _ = file.write_all(summary.as_bytes());
        eprintln!(
            "[스케줄러] daily_summary: {}건 대화 기록됨",
            conversation_count
        );
    }
}

/// [v0.8.0] 스케줄러 메인 루프 실행
/// 60초 간격으로 현재 시각과 크론 패턴을 매칭하여 해당 액션 실행.
/// shutdown_flag가 true가 되면 종료.
pub fn run_scheduler_loop(workspace: &Path, db_path: &Path, shutdown_flag: Arc<AtomicBool>) {
    eprintln!("[스케줄러] 시작됨 — workspace: {}", workspace.display());

    // schedule.toml 로드 (없으면 기본 생성)
    let _ = create_default_config(workspace);
    let config = match load_config(workspace) {
        Some(c) => c,
        None => {
            eprintln!("[스케줄러] schedule.toml 로드 실패");
            return;
        }
    };

    // 크론 패턴 사전 파싱
    let parsed: Vec<(ScheduleTask, CronPattern)> = config
        .tasks
        .into_iter()
        .filter_map(|t| CronPattern::parse(&t.cron).ok().map(|p| (t, p)))
        .collect();

    if parsed.is_empty() {
        eprintln!("[스케줄러] 유효한 예약 작업 없음");
        return;
    }

    eprintln!("[스케줄러] {}개 작업 등록됨", parsed.len());
    for (task, _) in &parsed {
        eprintln!("  - {} [{}] → {}", task.name, task.cron, task.action);
    }

    // 60초 간격 루프
    loop {
        if shutdown_flag.load(Ordering::Relaxed) {
            eprintln!("[스케줄러] 종료됨");
            break;
        }

        // 매칭 검사
        for (task, pattern) in &parsed {
            if pattern.matches_now() {
                eprintln!("[스케줄러] 실행: {} ({})", task.name, task.action);
                execute_action(&task.action, workspace, db_path);
            }
        }

        // 60초 대기 (1초 간격으로 shutdown 체크)
        for _ in 0..60 {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_field_any() {
        let f = CronField::parse("*").unwrap();
        assert!(f.matches(0));
        assert!(f.matches(59));
        assert!(f.matches(23));
    }

    #[test]
    fn test_cron_field_exact() {
        let f = CronField::parse("3").unwrap();
        assert!(f.matches(3));
        assert!(!f.matches(0));
        assert!(!f.matches(4));
    }

    #[test]
    fn test_cron_field_interval() {
        let f = CronField::parse("*/6").unwrap();
        assert!(f.matches(0));
        assert!(f.matches(6));
        assert!(f.matches(12));
        assert!(f.matches(18));
        assert!(!f.matches(3));
        assert!(!f.matches(7));
    }

    #[test]
    fn test_cron_pattern_parse() {
        let p = CronPattern::parse("0 3 * * *").unwrap();
        assert!(matches!(p.minute, CronField::Exact(0)));
        assert!(matches!(p.hour, CronField::Exact(3)));
        assert!(matches!(p.day, CronField::Any));
        assert!(matches!(p.month, CronField::Any));
        assert!(matches!(p.weekday, CronField::Any));
    }

    #[test]
    fn test_cron_pattern_parse_invalid() {
        assert!(CronPattern::parse("0 3 *").is_err());
        assert!(CronPattern::parse("").is_err());
    }

    #[test]
    fn test_schedule_config_parse() {
        let toml_str = r#"
[[tasks]]
name = "test"
cron = "0 3 * * *"
action = "memory_cleanup"
"#;
        let config: ScheduleConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].name, "test");
        assert_eq!(config.tasks[0].action, "memory_cleanup");
    }

    #[test]
    fn test_action_memory_cleanup() {
        let ws = std::env::temp_dir().join("femtoclaw_sched_test");
        std::fs::create_dir_all(&ws).ok();

        // 110줄 MEMORY.md 생성
        let memory_path = ws.join("MEMORY.md");
        let mut lines: Vec<String> = vec!["# MEMORY.md".to_string(), String::new()];
        for i in 0..110 {
            lines.push(format!(
                "- [2026-01-{:02} 10:00] 테스트 항목 {}",
                i % 28 + 1,
                i
            ));
        }
        std::fs::write(&memory_path, lines.join("\n") + "\n").unwrap();

        action_memory_cleanup(&ws);

        let after = std::fs::read_to_string(&memory_path).unwrap();
        let data_lines = after.lines().filter(|l| l.starts_with("- [")).count();
        assert_eq!(data_lines, 100);

        let _ = std::fs::remove_dir_all(&ws);
    }
}
