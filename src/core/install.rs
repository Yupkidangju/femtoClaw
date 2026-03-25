// femtoClaw — OS 네이티브 예약 등록/해제 모듈
// [v0.8.0] --install-schedule / --uninstall-schedule CLI 명령 구현
//
// 지원 OS:
//   - Windows: schtasks.exe (작업 스케줄러)
//   - Linux: crontab (사용자 크론탭)
//   - macOS: launchd (LaunchAgents plist)
//
// femtoclaw --run-schedule 명령을 OS가 정해진 시간에 자동 호출하도록 등록한다.

use std::path::Path;
use std::process::Command;

/// OS 네이티브 예약 작업 등록
/// 기본 매일 새벽 3시 실행 (schedule.toml의 내장 스케줄러가 세부 스케줄 관리)
pub fn install_schedule(exe_path: &Path) -> Result<String, String> {
    let exe = exe_path.to_str().ok_or("실행 파일 경로 변환 실패")?;

    #[cfg(target_os = "windows")]
    {
        install_windows(exe)
    }

    #[cfg(target_os = "linux")]
    {
        install_linux(exe)
    }

    #[cfg(target_os = "macos")]
    {
        install_macos(exe)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err("지원하지 않는 OS입니다".to_string())
    }
}

/// OS 네이티브 예약 작업 해제
pub fn uninstall_schedule() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        uninstall_windows()
    }

    #[cfg(target_os = "linux")]
    {
        uninstall_linux()
    }

    #[cfg(target_os = "macos")]
    {
        uninstall_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err("지원하지 않는 OS입니다".to_string())
    }
}

// ─── Windows: schtasks ───

#[cfg(target_os = "windows")]
fn install_windows(exe: &str) -> Result<String, String> {
    // schtasks /create /tn "femtoClaw" /tr "exe --run-schedule" /sc DAILY /st 03:00 /f
    let output = Command::new("schtasks")
        .args([
            "/create",
            "/tn",
            "femtoClaw",
            "/tr",
            &format!("\"{}\" --run-schedule", exe),
            "/sc",
            "DAILY",
            "/st",
            "03:00",
            "/f",
        ])
        .output()
        .map_err(|e| format!("schtasks 실행 실패: {}", e))?;

    if output.status.success() {
        Ok("✅ Windows 작업 스케줄러에 'femtoClaw' 등록 완료 (매일 03:00)".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("schtasks 등록 실패: {}", stderr))
    }
}

#[cfg(target_os = "windows")]
fn uninstall_windows() -> Result<String, String> {
    let output = Command::new("schtasks")
        .args(["/delete", "/tn", "femtoClaw", "/f"])
        .output()
        .map_err(|e| format!("schtasks 실행 실패: {}", e))?;

    if output.status.success() {
        Ok("✅ Windows 작업 스케줄러에서 'femtoClaw' 제거 완료".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("schtasks 제거 실패: {}", stderr))
    }
}

// ─── Linux: crontab ───

#[cfg(target_os = "linux")]
fn install_linux(exe: &str) -> Result<String, String> {
    let cron_entry = format!("0 3 * * * {} --run-schedule # femtoClaw", exe);

    // 기존 crontab 읽기
    let existing = Command::new("crontab")
        .arg("-l")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // 이미 등록된 경우 스킵
    if existing.contains("femtoClaw") {
        return Ok("ℹ️ crontab에 이미 등록되어 있습니다".to_string());
    }

    // 새 항목 추가
    let new_crontab = format!("{}{}\n", existing, cron_entry);

    // crontab에 쓰기 (파이프)
    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("crontab 실행 실패: {}", e))?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(new_crontab.as_bytes())
            .map_err(|e| format!("crontab 입력 실패: {}", e))?;
    }

    let status = child
        .wait()
        .map_err(|e| format!("crontab 대기 실패: {}", e))?;

    if status.success() {
        Ok("✅ crontab에 femtoClaw 등록 완료 (매일 03:00)".to_string())
    } else {
        Err("crontab 등록 실패".to_string())
    }
}

#[cfg(target_os = "linux")]
fn uninstall_linux() -> Result<String, String> {
    let existing = Command::new("crontab")
        .arg("-l")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let filtered: String = existing
        .lines()
        .filter(|l| !l.contains("femtoClaw"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("crontab 실행 실패: {}", e))?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(filtered.as_bytes())
            .map_err(|e| format!("crontab 입력 실패: {}", e))?;
    }

    let status = child
        .wait()
        .map_err(|e| format!("crontab 대기 실패: {}", e))?;

    if status.success() {
        Ok("✅ crontab에서 femtoClaw 제거 완료".to_string())
    } else {
        Err("crontab 제거 실패".to_string())
    }
}

// ─── macOS: launchd ───

#[cfg(target_os = "macos")]
fn install_macos(exe: &str) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없습니다")?;
    let agents_dir = home.join("Library/LaunchAgents");
    let plist_path = agents_dir.join("com.femtoclaw.schedule.plist");

    std::fs::create_dir_all(&agents_dir)
        .map_err(|e| format!("LaunchAgents 디렉토리 생성 실패: {}", e))?;

    // plist XML 생성
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.femtoclaw.schedule</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--run-schedule</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>3</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/femtoclaw-schedule.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/femtoclaw-schedule.err</string>
</dict>
</plist>
"#,
        exe
    );

    std::fs::write(&plist_path, plist_content)
        .map_err(|e| format!("plist 파일 생성 실패: {}", e))?;

    // launchctl load
    let output = Command::new("launchctl")
        .args(["load", plist_path.to_str().unwrap_or("")])
        .output()
        .map_err(|e| format!("launchctl 실행 실패: {}", e))?;

    if output.status.success() {
        Ok("✅ launchd에 femtoClaw 등록 완료 (매일 03:00)".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("launchctl 등록 실패: {}", stderr))
    }
}

#[cfg(target_os = "macos")]
fn uninstall_macos() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("홈 디렉토리를 찾을 수 없습니다")?;
    let plist_path = home.join("Library/LaunchAgents/com.femtoclaw.schedule.plist");

    if !plist_path.exists() {
        return Ok("ℹ️ launchd에 등록된 항목이 없습니다".to_string());
    }

    // launchctl unload
    let _ = Command::new("launchctl")
        .args(["unload", plist_path.to_str().unwrap_or("")])
        .output();

    std::fs::remove_file(&plist_path).map_err(|e| format!("plist 파일 삭제 실패: {}", e))?;

    Ok("✅ launchd에서 femtoClaw 제거 완료".to_string())
}
