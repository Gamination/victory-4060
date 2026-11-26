@echo off
setlocal EnableExtensions
rem ===========================================================================
rem  build.bat - builds victory from this checkout.
rem
rem    build.bat          -> target\diag\victory.exe     (what run.bat uses)
rem    build.bat release  -> target\release\victory.exe
rem    build.bat both     -> both
rem
rem  The diag build is a release build plus the diagnostics feature. The NVIDIA
rem  play settings in scripts\play-nvidia.ps1 need it: VICTORY_SKIP_PIPELINES
rem  (the ray-tracing shader list) is ignored by the plain release build.
rem
rem  Needs Rust (rust-toolchain.toml pins 1.98.1, rustup fetches it) and the
rem  MSVC C++ build tools with a Windows SDK. Only crates.io is downloaded.
rem  The first build of each profile compiles every dependency: 10-20 minutes.
rem ===========================================================================
cd /d "%~dp0"

set "WHAT=%~1"
if "%WHAT%"=="" set "WHAT=diag"
if /i not "%WHAT%"=="diag" if /i not "%WHAT%"=="release" if /i not "%WHAT%"=="both" (
  echo Usage: build.bat [diag^|release^|both]
  exit /b 2
)

where.exe cargo >nul 2>nul
if errorlevel 1 (
  echo cargo was not found in PATH.
  echo Install Rust from https://rustup.rs and run this again.
  set "CODE=1"
  goto :stop
)
if not exist "Cargo.lock" (
  echo Cargo.lock is missing: this does not look like the victory checkout.
  set "CODE=1"
  goto :stop
)

if /i "%WHAT%"=="both" (
  call :one diag
  if errorlevel 1 goto :failed
  call :one release
  if errorlevel 1 goto :failed
) else (
  call :one %WHAT%
  if errorlevel 1 goto :failed
)

echo.
if /i "%WHAT%"=="both" (
  echo Built: "%CD%\target\diag\victory.exe"
  echo        "%CD%\target\release\victory.exe"
) else (
  echo Built: "%CD%\target\%WHAT%\victory.exe"
)
echo Start the game with:  run.bat
set "CODE=0"
goto :stop

:failed
echo.
echo BUILD FAILED.
echo If the error mentions link.exe, MSVC or a Windows SDK, install the
echo Visual Studio Build Tools with "Desktop development with C++".
set "CODE=1"
goto :stop

:one
set "PROFILE=%~1"
set "FEATURES="
if /i "%PROFILE%"=="diag" set "FEATURES=--features diagnostics"
echo.
echo === victory, %PROFILE% profile ===
cargo build --profile %PROFILE% --locked -p victory %FEATURES%
if errorlevel 1 exit /b 1
exit /b 0

:stop
rem VICTORY_NO_PAUSE=1 skips the wait (shortcuts, scripts).
if not defined VICTORY_NO_PAUSE pause
exit /b %CODE%
