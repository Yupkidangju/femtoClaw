# 변경 이력 (Changelog)

이 프로젝트의 모든 주요 변경사항은 이 파일에 기록됩니다.
형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를 따르며,
버전 관리는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 준수합니다.

## [0.1.0-beta] - 2026-03-23

### 추가됨 (Added)
- 프로젝트 초기 스펙 문서(`spec.md`) 작성 완료
- 인터페이스 디자인 청사진(`designs.md`) 작성 완료
- 디자인 방향 확정: **Design 2 — Amber Monochrome** (Midnight Commander 스타일)
- D3D Protocol 기반 전체 문서 체계 수립 (9개 필수 문서)
- LLM Provider 전략 확립: 2-Format (OpenAI-Compatible + Ollama), 7개 프리셋
- 정적 파일 기반 스킬 시스템 설계 (TOML/JSON)
- Headless 모드 상세 사양 추가
- 에러/장애 복구 시나리오 정의 (Exponential Backoff, DB 복구, 오프라인 큐잉)
- 비밀번호 3회 실패 리셋 정책 추가
- 버전 로드맵 정의 (v0.1 → v0.2 Rhai → v0.3 멀티 에이전트)
- **[Step 1] 코어 샌드박스 초기화** — 디렉토리 구조, 프로세스 락, config.enc 암호화/복호화
- **[Step 2] TUI 온보딩 & 검증** — Amber Monochrome 테마, 4개 화면(Boot→PW→Onboard→Dashboard)
- **[Step 2] API 키 비동기 검증** — reqwest 별도 스레드 + mpsc 채널 (TUI 비블로킹)
- **[Step 2] 모델 자동 선택기** — /models 파싱 → ↑↓ 방향키로 모델 순환 선택
- **[Step 2] 입력 방어 로직** — 글자 수 상한(PW 128, Key 256, Model 64, Token 128) + 표시 잘림
- **[Step 2] Windows KeyEvent 필터** — `KeyEventKind::Press`만 처리 (Release/Repeat 무시)
- **[Step 2] 대시보드 메뉴 스텁** — [1]-[4] 키 피드백 응답

### 변경됨 (Changed)
- spec.md 전면 개정: v0.1 경량화 방향 반영
- designs.md 전면 개정: Design 2 기준, 단일 에이전트 UI, 간소 Undo
- 에이전트 수: 최대 3개 → **1개** (v0.1)
- 타임머신: 풀 데이터 그리드 → **간소 Undo** (최근 5건 + 마지막 취소)
- 부트 시퀀스: 매번 풀 연출 → **빠른 부트 기본** (0.5초)
- LLM 공급자: 7개 개별 구현 → **2개 API 형식 + 7개 프리셋**
- **HTTP 타임아웃: 10초 → 5초** (connect 3초 + total 5초)
- **검증 재시도: Testing 중에도 [V] 재입력 허용**

### 제거됨 (Removed)
- Rhai 동적 스크립팅 엔진 (v0.2 이후 검토)
- 멀티 에이전트 라우팅 로직 (v0.3 이후 검토)

### 참고사항
- 현재 단계: **Step 1 완료, Step 2 완료** (빌드/테스트 통과, 사용자 검증 완료)
- ZSTD 압축은 라즈베리 파이 32GB 생존을 위해 유지 (저장 수명 20배 연장)
- 다음 구현 대상: Step 3 (Database & Compression Engine)

