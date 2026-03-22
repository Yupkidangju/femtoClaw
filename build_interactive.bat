@echo off
REM femtoClaw — 대화형 빌드 스크립트 (Windows)
REM [v0.1.0] 개발자가 메뉴를 선택하여 빌드하는 인터랙티브 모드

setlocal enabledelayedexpansion
chcp 65001 >nul 2>&1
cls

echo.
echo  +----------------------------------------------+
echo  ^|  femtoClaw Build System v0.1.0 (Interactive) ^|
echo  +----------------------------------------------+
echo.
echo   [1] Release 빌드 (최적화)
echo   [2] Debug 빌드 (개발용)
echo   [3] 테스트 실행
echo   [4] Clippy 린트 검사
echo   [5] 빌드 캐시 정리
echo   [0] 종료
echo.

set /p "CHOICE=선택 (0-5): "

if "%CHOICE%"=="1" goto :release
if "%CHOICE%"=="2" goto :debug
if "%CHOICE%"=="3" goto :test
if "%CHOICE%"=="4" goto :clippy
if "%CHOICE%"=="5" goto :clean
if "%CHOICE%"=="0" goto :exit_end

echo [오류] 잘못된 선택: %CHOICE%
goto :exit_end

:release
echo.
echo [INFO] Release 빌드 시작...
call build.bat
goto :done

:debug
echo.
echo [INFO] Debug 빌드 시작...
call build.bat --debug
goto :done

:test
echo.
echo [INFO] 테스트 실행...
call build.bat --test
goto :done

:clippy
echo.
echo [INFO] Clippy 린트 검사 중...
cargo clippy -- -W warnings
if errorlevel 1 (
    echo [FAIL] Clippy 경고 발견
) else (
    echo [  OK] 린트 통과
)
goto :done

:clean
echo.
echo [INFO] 빌드 캐시 정리...
call build.bat --clean
goto :done

:done
echo.
echo === 프로세스 완료 ===
pause
goto :exit_end

:exit_end
endlocal
