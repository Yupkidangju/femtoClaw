@echo off
REM femtoClaw — Windows 빌드 스크립트 (명령형/비대화식)
REM [v0.1.0] CI/CD 및 자동화용
REM
REM 사용법:
REM   build.bat                  # 릴리즈 빌드
REM   build.bat --debug          # 디버그 빌드
REM   build.bat --test           # 테스트만 실행
REM   build.bat --clean          # 빌드 캐시 정리

setlocal enabledelayedexpansion
chcp 65001 >nul 2>&1

set "BUILD_TYPE=release"
set "ACTION=build"
set "OUTPUT_DIR=dist"

REM === 인자 파싱 ===
:parse_args
if "%~1"=="" goto :main
if "%~1"=="--debug" (
    set "BUILD_TYPE=debug"
    shift
    goto :parse_args
)
if "%~1"=="--test" (
    set "ACTION=test"
    shift
    goto :parse_args
)
if "%~1"=="--clean" (
    set "ACTION=clean"
    shift
    goto :parse_args
)
if "%~1"=="--help" goto :show_help
if "%~1"=="-h" goto :show_help
echo [FAIL] 알 수 없는 옵션: %~1
exit /b 1

:show_help
echo 사용법: build.bat [옵션]
echo.
echo 옵션:
echo   --debug    디버그 빌드
echo   --test     테스트만 실행
echo   --clean    빌드 캐시 정리
echo   --help     이 도움말
exit /b 0

:main
echo.
echo +------------------------------------------+
echo ^|  femtoClaw Build System v0.1.0           ^|
echo +------------------------------------------+
echo.

if "%ACTION%"=="clean" goto :do_clean
if "%ACTION%"=="test" goto :do_test
goto :do_build

:do_clean
echo [INFO] 빌드 캐시 정리 중...
cargo clean
if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"
echo [  OK] 완료
goto :end

:do_test
echo [INFO] 테스트 실행 중...
cargo test
if errorlevel 1 (
    echo [FAIL] 테스트 실패
    exit /b 1
)
echo [  OK] 모든 테스트 통과
goto :end

:do_build
echo [INFO] Windows 릴리즈 빌드 중...

if "%BUILD_TYPE%"=="release" (
    cargo build --release
) else (
    cargo build
)

if errorlevel 1 (
    echo [FAIL] 빌드 실패
    exit /b 1
)

REM 바이너리 복사
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

if "%BUILD_TYPE%"=="release" (
    copy /Y "target\release\femtoclaw.exe" "%OUTPUT_DIR%\femtoclaw-windows-x64.exe" >nul
) else (
    copy /Y "target\debug\femtoclaw.exe" "%OUTPUT_DIR%\femtoclaw-windows-x64.exe" >nul
)

for %%A in ("%OUTPUT_DIR%\femtoclaw-windows-x64.exe") do set "SIZE=%%~zA"
set /a "SIZE_KB=!SIZE! / 1024"
echo [  OK] 완료: %OUTPUT_DIR%\femtoclaw-windows-x64.exe (!SIZE_KB! KB)

:end
echo.
echo [  OK] 빌드 완료!
endlocal
