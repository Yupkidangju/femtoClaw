#!/bin/bash
# femtoClaw — 대화형 빌드 스크립트 (Linux/macOS/Raspberry Pi)
# [v0.1.0] 개발자가 메뉴를 선택하여 빌드하는 인터랙티브 모드

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
AMBER='\033[38;2;255;176;0m'
NC='\033[0m'

clear
echo -e "${AMBER}"
echo "┌──────────────────────────────────────────────┐"
echo "│  ███████╗███████╗███╗   ███╗████████╗ ██████╗│"
echo "│  ██╔════╝██╔════╝████╗ ████║╚══██╔══╝██╔═══█║"
echo "│  █████╗  █████╗  ██╔████╔██║   ██║   ██║   █║"
echo "│  ██╔══╝  ██╔══╝  ██║╚██╔╝██║   ██║   ██║   █║"
echo "│  ██║     ███████╗██║ ╚═╝ ██║   ██║   ╚██████║"
echo "│  ╚═╝     ╚══════╝╚═╝     ╚═╝   ╚═╝    ╚═════╝"
echo "│            femtoClaw Build System v0.1.0      │"
echo "└──────────────────────────────────────────────┘"
echo -e "${NC}"

echo -e "빌드 타겟을 선택하세요:\n"
echo -e "  ${CYAN}[1]${NC} 현재 시스템 (native)"
echo -e "  ${CYAN}[2]${NC} Linux x86_64"
echo -e "  ${CYAN}[3]${NC} Raspberry Pi (ARM 32-bit)"
echo -e "  ${CYAN}[4]${NC} Raspberry Pi (ARM 64-bit)"
echo -e "  ${CYAN}[5]${NC} 모든 타겟 빌드"
echo -e "  ${CYAN}[6]${NC} 테스트만 실행"
echo -e "  ${CYAN}[7]${NC} 빌드 캐시 정리"
echo -e "  ${CYAN}[0]${NC} 종료"
echo ""
read -p "선택 (0-7): " choice

case "$choice" in
    1) bash build.sh --target native ;;
    2) bash build.sh --target linux ;;
    3) bash build.sh --target rpi ;;
    4) bash build.sh --target rpi64 ;;
    5) bash build.sh --target all ;;
    6) bash build.sh --test ;;
    7) bash build.sh --clean ;;
    0) echo "종료합니다."; exit 0 ;;
    *)
        echo -e "${RED}[오류] 잘못된 선택: $choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}━━━ 빌드 프로세스 완료 ━━━${NC}"
read -p "Enter를 누르면 종료합니다..."
