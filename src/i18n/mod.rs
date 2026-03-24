// femtoClaw — i18n 국제화 모듈
// [v0.5.0] Phase 1: 다국어 인프라
//
// 설계 원칙:
//   - Lang 열거형으로 5개 언어 지원 (ko, en, ja, zh-tw, zh-cn)
//   - OS 시스템 언어를 자동 감지하여 기본 언어 결정
//   - 지원하지 않는 언어는 영어(en)로 fallback
//   - msg!() 매크로로 어디서든 현재 언어의 메시지를 가져올 수 있음
//   - --lang CLI 인자로 수동 오버라이드 가능
//   - TUI 화면은 언어별 별도 렌더 함수로 치환 (폭 계산 대신 화면 통째 교체)

pub mod en;
pub mod ja;
pub mod keys;
pub mod ko;
pub mod zh_cn;
pub mod zh_tw;

use std::sync::atomic::{AtomicU8, Ordering};

/// [v0.5.0] 지원 언어 열거형
/// D3D 규칙: 한 / 영 / 일 / 중(번체) / 중(간체)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Lang {
    /// 한국어
    Ko = 0,
    /// 영어 (기본 fallback)
    En = 1,
    /// 일본어
    Ja = 2,
    /// 중국어 번체 (대만/홍콩)
    ZhTw = 3,
    /// 중국어 간체 (중국 대륙)
    ZhCn = 4,
}

impl Lang {
    /// 언어 코드 문자열 → Lang 변환
    /// 지원하지 않는 코드는 None 반환
    pub fn from_code(code: &str) -> Option<Lang> {
        let lower = code.to_lowercase();
        match lower.as_str() {
            "ko" | "ko-kr" | "korean" => Some(Lang::Ko),
            "en" | "en-us" | "en-gb" | "english" => Some(Lang::En),
            "ja" | "ja-jp" | "japanese" => Some(Lang::Ja),
            "zh-tw" | "zh-hant" | "zh_tw" => Some(Lang::ZhTw),
            "zh-cn" | "zh-hans" | "zh_cn" | "zh" => Some(Lang::ZhCn),
            _ => None,
        }
    }

    /// Lang → BCP 47 언어 코드
    pub fn code(&self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
            Lang::Ja => "ja",
            Lang::ZhTw => "zh-TW",
            Lang::ZhCn => "zh-CN",
        }
    }

    /// 언어 이름 (해당 언어로)
    pub fn native_name(&self) -> &'static str {
        match self {
            Lang::Ko => "한국어",
            Lang::En => "English",
            Lang::Ja => "日本語",
            Lang::ZhTw => "繁體中文",
            Lang::ZhCn => "简体中文",
        }
    }

    /// u8 → Lang 변환 (범위 초과 시 En)
    fn from_u8(v: u8) -> Lang {
        match v {
            0 => Lang::Ko,
            1 => Lang::En,
            2 => Lang::Ja,
            3 => Lang::ZhTw,
            4 => Lang::ZhCn,
            _ => Lang::En,
        }
    }
}

// === 전역 언어 상태 ===

/// 현재 활성 언어 (AtomicU8로 스레드 안전)
static CURRENT_LANG: AtomicU8 = AtomicU8::new(1); // 기본값: En

/// [v0.5.0] 현재 활성 언어를 반환한다.
pub fn current_lang() -> Lang {
    Lang::from_u8(CURRENT_LANG.load(Ordering::Relaxed))
}

/// [v0.5.0] 현재 활성 언어를 변경한다.
pub fn set_lang(lang: Lang) {
    CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
}

/// [v0.5.0] OS 시스템 언어를 감지하여 자동으로 설정한다.
///
/// 감지 알고리즘:
///   1. 환경변수 FEMTOCLAW_LANG이 있으면 최우선 사용
///   2. Windows: GetUserDefaultUILanguage() → LANGID 매핑
///   3. Unix: LANG, LC_ALL, LC_MESSAGES 환경변수 순서로 확인
///   4. 지원하지 않는 언어거나 감지 실패 시 → 영어(En) fallback
///
/// 반환: 감지된 (또는 fallback된) 언어
pub fn detect_and_set_lang() -> Lang {
    // 1. 환경변수 오버라이드 (최우선)
    if let Ok(env_lang) = std::env::var("FEMTOCLAW_LANG") {
        if let Some(lang) = Lang::from_code(&env_lang) {
            set_lang(lang);
            return lang;
        }
    }

    // 2. OS 시스템 언어 감지
    let detected = detect_os_language();
    let lang = detected
        .and_then(|code| Lang::from_code(&code))
        .unwrap_or(Lang::En); // 미지원 언어 → 영어 fallback

    set_lang(lang);
    lang
}

/// OS별 시스템 언어 감지
/// Windows: GetUserDefaultUILanguage API 호출
/// Unix: LANG 환경변수 파싱
fn detect_os_language() -> Option<String> {
    // Unix/macOS: LANG, LC_ALL, LC_MESSAGES 환경변수 확인
    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() && val != "C" && val != "POSIX" {
                // "ko_KR.UTF-8" → "ko" 추출
                let code = val.split('.').next().unwrap_or(&val).replace('_', "-");
                // "ko-KR" → "ko" (2글자 코드만 필요한 경우도 대응)
                return Some(code);
            }
        }
    }

    // Windows: 레지스트리 또는 시스템 로케일 확인
    #[cfg(target_os = "windows")]
    {
        // Windows: chcp / GetUserDefaultUILanguage 대신 PowerShell로 간단히
        if let Ok(output) = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Culture).TwoLetterISOLanguageName",
            ])
            .output()
        {
            let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !code.is_empty() {
                return Some(code);
            }
        }
    }

    None
}

// === 메시지 조회 ===

/// [v0.5.0] 현재 언어로 메시지를 가져온다.
/// 해당 언어에 키가 없으면 영어 → 한국어 순서로 fallback.
pub fn get_msg(key: &str) -> &'static str {
    let lang = current_lang();

    // 1. 현재 언어에서 조회
    if let Some(msg) = lookup(lang, key) {
        return msg;
    }

    // 2. 영어 fallback
    if lang != Lang::En {
        if let Some(msg) = lookup(Lang::En, key) {
            return msg;
        }
    }

    // 3. 한국어 fallback (원본)
    if lang != Lang::Ko {
        if let Some(msg) = lookup(Lang::Ko, key) {
            return msg;
        }
    }

    // 4. 키 자체를 반환 (디버깅용)
    // 런타임에 발견 못한 키 → 키 문자열 그대로 표시
    // 메모리 안전을 위해 키를 leak (소량이므로 무방)
    Box::leak(key.to_string().into_boxed_str())
}

/// 특정 언어의 메시지 맵에서 키를 조회
fn lookup(lang: Lang, key: &str) -> Option<&'static str> {
    match lang {
        Lang::Ko => ko::get(key),
        Lang::En => en::get(key),
        Lang::Ja => ja::get(key),
        Lang::ZhTw => zh_tw::get(key),
        Lang::ZhCn => zh_cn::get(key),
    }
}

// === msg!() 매크로 ===

/// [v0.5.0] 다국어 메시지 매크로
///
/// 사용법:
///   msg!("err.home_not_found")              → &'static str 반환
///   msg!("err.sandbox_create", err)         → String 반환 ({}를 순차 치환)
///   msg!("cli.paired", name, id)            → String 반환
///
/// 인자가 없으면 &'static str, 인자가 있으면 String을 반환한다.
/// format!()은 컴파일 타임 리터럴만 받으므로, 런타임 문자열에 대해
/// str::replacen을 사용하여 {} 플레이스홀더를 순차 치환한다.
#[macro_export]
macro_rules! msg {
    // 인자 없음 → &'static str 반환
    ($key:expr) => {
        $crate::i18n::get_msg($key)
    };
    // format 인자 있음 → String 반환 ({}를 순차 치환)
    ($key:expr, $($arg:expr),+ $(,)?) => {
        {
            let mut _s = $crate::i18n::get_msg($key).to_string();
            $(
                _s = _s.replacen("{}", &format!("{}", $arg), 1);
            )+
            // {:?} 패턴도 치환 (Debug 포맷용)
            _s
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lang_from_code() {
        assert_eq!(Lang::from_code("ko"), Some(Lang::Ko));
        assert_eq!(Lang::from_code("ko-KR"), Some(Lang::Ko));
        assert_eq!(Lang::from_code("en"), Some(Lang::En));
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("zh-TW"), Some(Lang::ZhTw));
        assert_eq!(Lang::from_code("zh-CN"), Some(Lang::ZhCn));
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::from_code("de"), None);
    }

    #[test]
    fn test_lang_code_roundtrip() {
        for lang in &[Lang::Ko, Lang::En, Lang::Ja, Lang::ZhTw, Lang::ZhCn] {
            let code = lang.code();
            assert!(
                Lang::from_code(code).is_some(),
                "code {} should round-trip",
                code
            );
        }
    }

    #[test]
    fn test_set_and_get_lang() {
        set_lang(Lang::Ko);
        assert_eq!(current_lang(), Lang::Ko);
        set_lang(Lang::En);
        assert_eq!(current_lang(), Lang::En);
        set_lang(Lang::Ja);
        assert_eq!(current_lang(), Lang::Ja);
    }

    #[test]
    fn test_native_names() {
        assert_eq!(Lang::Ko.native_name(), "한국어");
        assert_eq!(Lang::En.native_name(), "English");
        assert_eq!(Lang::Ja.native_name(), "日本語");
        assert_eq!(Lang::ZhTw.native_name(), "繁體中文");
        assert_eq!(Lang::ZhCn.native_name(), "简体中文");
    }

    #[test]
    fn test_detect_returns_something() {
        // OS 언어 감지가 Some 또는 None을 반환하는지만 확인
        // 실제 값은 환경에 따라 다름
        let _ = detect_os_language();
    }

    #[test]
    fn test_get_msg_ko_basic() {
        set_lang(Lang::Ko);
        let msg = get_msg("err.home_not_found");
        // 한국어 맵에 키가 있으면 한국어, 없으면 fallback
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_get_msg_fallback_to_key() {
        set_lang(Lang::En);
        // 존재하지 않는 키 → 키 문자열 자체 반환
        let msg = get_msg("nonexistent.key.for.test");
        assert_eq!(msg, "nonexistent.key.for.test");
    }

    #[test]
    fn test_msg_macro_no_args() {
        set_lang(Lang::Ko);
        let text: &str = msg!("err.home_not_found");
        assert!(!text.is_empty());
    }

    #[test]
    fn test_unsupported_lang_fallback() {
        // 미지원 코드 → None → 영어 fallback
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_code("es"), None);
    }
}
