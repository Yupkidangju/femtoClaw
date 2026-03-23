#!/bin/bash
# femtoClaw — 크로스 플랫폼 빌드 스크립트 (Linux/macOS/Raspberry Pi)
# [v0.4.0] 명령형 (비대화식) — CI/CD 및 자동화용
#
# 사용법:
#   ./build.sh                    # 현재 타겟 릴리즈 빌드
#   ./build.sh --target linux     # Linux x86_64
#   ./build.sh --target rpi       # Raspberry Pi (ARM)
#   ./build.sh --target all       # 모든 타겟
#   ./build.sh --debug            # 디버그 빌드
#   ./build.sh --test             # 테스트만 실행
#   ./build.sh --clean            # 빌드 캐시 정리

set -euo pipefail

# === 색상 정의 ===
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "${CYAN}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[  OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()   { echo -e "${RED}[FAIL]${NC} $1"; }

# === 기본값 ===
TARGET="native"
BUILD_TYPE="release"
ACTION="build"
OUTPUT_DIR="dist"

# === 타겟 트리플 정의 ===
TARGET_LINUX="x86_64-unknown-linux-gnu"
TARGET_RPI="armv7-unknown-linux-gnueabihf"
TARGET_RPI64="aarch64-unknown-linux-gnu"

# === 인자 파싱 ===
while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --test)
            ACTION="test"
            shift
            ;;
        --clean)
            ACTION="clean"
            shift
            ;;
        --help|-h)
            echo "사용법: $0 [옵션]"
            echo ""
            echo "옵션:"
            echo "  --target <linux|rpi|rpi64|all>  빌드 타겟 (기본: native)"
            echo "  --debug                          디버그 빌드"
            echo "  --test                           테스트만 실행"
            echo "  --clean                          빌드 캐시 정리"
            echo "  --help                           이 도움말"
            exit 0
            ;;
        *)
            log_err "알 수 없는 옵션: $1"
            exit 1
            ;;
    esac
done

# === 함수: 단일 타겟 빌드 ===
build_target() {
    local triple="$1"
    local label="$2"

    log_info "빌드 중: ${label} (${triple})"

    # 크로스 컴파일 타겟 설치 확인
    if [[ "$triple" != "native" ]]; then
        if ! rustup target list --installed | grep -q "$triple"; then
            log_info "타겟 추가: $triple"
            rustup target add "$triple"
        fi
    fi

    # 빌드 실행
    local cargo_args=()
    if [[ "$BUILD_TYPE" == "release" ]]; then
        cargo_args+=(--release)
    fi
    if [[ "$triple" != "native" ]]; then
        cargo_args+=(--target "$triple")
    fi

    cargo build "${cargo_args[@]}"

    # 바이너리 복사
    mkdir -p "$OUTPUT_DIR"
    local src_path
    if [[ "$triple" == "native" ]]; then
        src_path="target/${BUILD_TYPE}/femtoclaw"
    else
        src_path="target/${triple}/${BUILD_TYPE}/femtoclaw"
    fi

    if [[ -f "$src_path" ]]; then
        local dest="$OUTPUT_DIR/femtoclaw-${label}"
        cp "$src_path" "$dest"
        chmod +x "$dest"
        local size=$(du -h "$dest" | cut -f1)
        log_ok "완료: ${dest} (${size})"
    else
        log_warn "바이너리 미생성: $src_path"
    fi
}

# === 실행 ===
echo ""
echo "┌──────────────────────────────────────────┐"
echo "│  femtoClaw Build System v0.4.0           │"
echo "└──────────────────────────────────────────┘"
echo ""

case "$ACTION" in
    clean)
        log_info "빌드 캐시 정리 중..."
        cargo clean
        rm -rf "$OUTPUT_DIR"
        log_ok "완료"
        ;;
    test)
        log_info "테스트 실행 중..."
        cargo test
        log_ok "모든 테스트 통과"
        ;;
    build)
        case "$TARGET" in
            native)
                build_target "native" "native"
                ;;
            linux)
                build_target "$TARGET_LINUX" "linux-x64"
                ;;
            rpi)
                build_target "$TARGET_RPI" "rpi-armv7"
                ;;
            rpi64)
                build_target "$TARGET_RPI64" "rpi-aarch64"
                ;;
            all)
                build_target "native" "native"
                build_target "$TARGET_LINUX" "linux-x64"
                build_target "$TARGET_RPI64" "rpi-aarch64"
                ;;
            *)
                log_err "알 수 없는 타겟: $TARGET"
                exit 1
                ;;
        esac
        echo ""
        log_ok "빌드 완료! 출력: $OUTPUT_DIR/"
        ;;
esac
