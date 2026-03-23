@echo off
REM femtoClaw Build Script for Windows
REM [v0.4.0] Non-interactive, CI/CD ready
REM
REM Usage:
REM   build.bat                  # Release build
REM   build.bat --debug          # Debug build
REM   build.bat --test           # Run tests only
REM   build.bat --clean          # Clean build cache

setlocal enabledelayedexpansion

set "BUILD_TYPE=release"
set "ACTION=build"
set "OUTPUT_DIR=dist"

REM === Parse arguments ===
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
echo [FAIL] Unknown option: %~1
exit /b 1

:show_help
echo Usage: build.bat [options]
echo.
echo Options:
echo   --debug    Debug build
echo   --test     Run tests only
echo   --clean    Clean build cache
echo   --help     Show this help
exit /b 0

:main
echo.
echo +------------------------------------------+
echo ^|  femtoClaw Build System v0.4.0           ^|
echo +------------------------------------------+
echo.

if "%ACTION%"=="clean" goto :do_clean
if "%ACTION%"=="test" goto :do_test
goto :do_build

:do_clean
echo [INFO] Cleaning build cache...
cargo clean
if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"
echo [  OK] Done
goto :end

:do_test
echo [INFO] Running tests...
cargo test
if errorlevel 1 (
    echo [FAIL] Tests failed
    exit /b 1
)
echo [  OK] All tests passed
goto :end

:do_build
echo [INFO] Building Windows release...

if "%BUILD_TYPE%"=="release" (
    cargo build --release
) else (
    cargo build
)

if errorlevel 1 (
    echo [FAIL] Build failed
    exit /b 1
)

REM Copy binary
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

if "%BUILD_TYPE%"=="release" (
    copy /Y "target\release\femtoclaw.exe" "%OUTPUT_DIR%\femtoclaw-windows-x64.exe" >nul
) else (
    copy /Y "target\debug\femtoclaw.exe" "%OUTPUT_DIR%\femtoclaw-windows-x64.exe" >nul
)

for %%A in ("%OUTPUT_DIR%\femtoclaw-windows-x64.exe") do set "SIZE=%%~zA"
set /a "SIZE_KB=!SIZE! / 1024"
echo [  OK] Done: %OUTPUT_DIR%\femtoclaw-windows-x64.exe (!SIZE_KB! KB)

:end
echo.
echo [  OK] Build complete!
endlocal
