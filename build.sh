#!/bin/bash
# femtoClaw — Cross-platform build script (Linux/macOS/Raspberry Pi)
# [v0.4.0] Non-interactive, CI/CD ready
#
# Usage:
#   ./build.sh                    # Native release build
#   ./build.sh --target linux     # Linux x86_64
#   ./build.sh --target rpi       # Raspberry Pi (ARM)
#   ./build.sh --target all       # All targets
#   ./build.sh --debug            # Debug build
#   ./build.sh --test             # Run tests only
#   ./build.sh --clean            # Clean build cache

set -euo pipefail

# === Color definitions ===
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "${CYAN}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[  OK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_err()   { echo -e "${RED}[FAIL]${NC} $1"; }

# === Defaults ===
TARGET="native"
BUILD_TYPE="release"
ACTION="build"
OUTPUT_DIR="dist"

# === Target triples ===
TARGET_LINUX="x86_64-unknown-linux-gnu"
TARGET_RPI="armv7-unknown-linux-gnueabihf"
TARGET_RPI64="aarch64-unknown-linux-gnu"

# === Parse arguments ===
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
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --target <linux|rpi|rpi64|all>  Build target (default: native)"
            echo "  --debug                          Debug build"
            echo "  --test                           Run tests only"
            echo "  --clean                          Clean build cache"
            echo "  --help                           Show this help"
            exit 0
            ;;
        *)
            log_err "Unknown option: $1"
            exit 1
            ;;
    esac
done

# === Function: build a single target ===
build_target() {
    local triple="$1"
    local label="$2"

    log_info "Building: ${label} (${triple})"

    # Install cross-compile target if needed
    if [[ "$triple" != "native" ]]; then
        if ! rustup target list --installed | grep -q "$triple"; then
            log_info "Adding target: $triple"
            rustup target add "$triple"
        fi
    fi

    # Build
    local cargo_args=()
    if [[ "$BUILD_TYPE" == "release" ]]; then
        cargo_args+=(--release)
    fi
    if [[ "$triple" != "native" ]]; then
        cargo_args+=(--target "$triple")
    fi

    cargo build "${cargo_args[@]}"

    # Copy binary
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
        local size
        size=$(du -h "$dest" | cut -f1)
        log_ok "Done: ${dest} (${size})"
    else
        log_warn "Binary not found: $src_path"
    fi
}

# === Execute ===
echo ""
echo "+-----------------------------------------+"
echo "|  femtoClaw Build System v0.4.0          |"
echo "+-----------------------------------------+"
echo ""

case "$ACTION" in
    clean)
        log_info "Cleaning build cache..."
        cargo clean
        rm -rf "$OUTPUT_DIR"
        log_ok "Done"
        ;;
    test)
        log_info "Running tests..."
        cargo test
        log_ok "All tests passed"
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
                log_err "Unknown target: $TARGET"
                exit 1
                ;;
        esac
        echo ""
        log_ok "Build complete! Output: $OUTPUT_DIR/"
        ;;
esac
