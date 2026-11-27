@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem ===========================================================================
rem  run.bat - starts the title using the NVIDIA or Radeon profile.
rem
rem    run.bat              Series S, rendering at 540p (what this fork is
rem                         measured with: about 20 fps on a 4060)
rem    run.bat amd          use the Radeon launcher
rem    run.bat amd streaming fsr3     Radeon streaming and native FSR 3 upscaling
rem    run.bat streaming fsr3         equivalent options on the NVIDIA profile
rem    run.bat 720p         render at 720p instead (480p, 540p, 1080p, 1440p,
rem                         2160p also accepted)
rem    run.bat quality      fewer stretched bodies and less rainbow skin,
rem                         about 7 fps slower
rem    run.bat rt           keep the title's ray-tracing passes (on NVIDIA
rem                         rooms go white: the builder cannot run there)
rem    run.bat seriesx      the Series X profile instead of Series S
rem    run.bat release      use target\release instead of target\diag
rem                         (no ray-tracing skip list there)
rem  Every launch translates the title's shaders and makes the pipelines a
rem  place needs before starting it, and puts back what "save" wrote, so the
rem  first time a place is seen is not slow while they are made. Reading the
rem  title's files is the slow part, so it is done only when something changed
rem  since the last launch (the build, the settings, a file, the cache).
rem    run.bat precompile   walk the title even if nothing changed since the
rem                         last precompile
rem    run.bat skip         start it without precompiling at all (the quick
rem                         launch, for when the caches are made already)
rem    run.bat save         precompile, then write every translated shader into
rem                         precompiled.vpc beside the cache folder: one file,
rem                         for a cache that is wiped (or kept elsewhere)
rem    run.bat load         put that file back into the cache, then precompile
rem    run.bat path         ask for the title's .exe again (a moved copy, or
rem                         another dump); the choice is kept for later runs
rem
rem  The first run asks for the title's .exe with a file dialog; if it is
rem  cancelled, paste the path into this window or drag the file into it. The
rem  path is kept in %LOCALAPPDATA%\victory\game-path.txt and used from then
rem  on: change the dump, or move it, and the next run asks again by itself.
rem  These combine:  run.bat precompile quality 720p
rem  Set VICTORY_NO_PAUSE=1 to
rem  skip the wait, VICTORY_DRY_RUN=1 to only print what would start.
rem ===========================================================================
cd /d "%~dp0"

set "BUILD=diag"
set "PROFILE=nvidia"
set "STREAMING="
set "NVONLY="
set "ARGS="
set "CHOOSE="

:args
if "%~1"=="" goto :args_done
if /i "%~1"=="bar" ( set "VICTORY_CARD_BAR=1" & set "VICTORY_READBACK_BAR=1" & shift & goto :args )
if /i "%~1"=="nobar" ( set "VICTORY_CARD_BAR=0" & set "VICTORY_READBACK_BAR=0" & shift & goto :args )
if /i "%~1"=="amd" ( set "PROFILE=amd" & shift & goto :args )
if /i "%~1"=="streaming" ( set "STREAMING=1" & shift & goto :args )
if /i "%~1"=="fsr3" ( set "ARGS=%ARGS% -NativeFSR3" & shift & goto :args )
if /i "%~1"=="quality"  ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Quality"  & shift & goto :args )
if /i "%~1"=="rt"       ( set "ARGS=%ARGS% -RayTracing" & shift & goto :args )
if /i "%~1"=="seriesx"  ( set "NVONLY=%~1" & set "ARGS=%ARGS% -SeriesX"  & shift & goto :args )
if /i "%~1"=="seriess"  ( shift & goto :args )
if /i "%~1"=="release"  ( set "BUILD=release" & shift & goto :args )
if /i "%~1"=="diag"     ( set "BUILD=diag" & shift & goto :args )
if /i "%~1"=="precompile" ( set "ARGS=%ARGS% -Precompile" & shift & goto :args )
if /i "%~1"=="skip"     ( set "ARGS=%ARGS% -NoPrecompile" & shift & goto :args )
if /i "%~1"=="save"     ( set "ARGS=%ARGS% -SaveShaders" & shift & goto :args )
if /i "%~1"=="load"     ( set "ARGS=%ARGS% -LoadShaders" & shift & goto :args )
if /i "%~1"=="path"     ( set "CHOOSE=1" & shift & goto :args )
if /i "%~1"=="480p"     ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 480"  & shift & goto :args )
if /i "%~1"=="540p"     ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 540"  & shift & goto :args )
if /i "%~1"=="720p"     ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 720"  & shift & goto :args )
if /i "%~1"=="1080p"    ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 1080" & shift & goto :args )
if /i "%~1"=="1440p"    ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 1440" & shift & goto :args )
if /i "%~1"=="2160p"    ( set "NVONLY=%~1" & set "ARGS=%ARGS% -Resolution 2160" & shift & goto :args )
echo Unknown option "%~1".
echo Options: bar, nobar, amd, streaming, fsr3, path, precompile, skip, save, load, quality, rt, seriesx, release, diag, 480p, 540p, 720p, 1080p, 1440p, 2160p
exit /b 2
:args_done
if defined STREAMING set "ARGS=%ARGS% -TextureStreaming"
if "%PROFILE%"=="amd" if defined NVONLY (
  echo Option "%NVONLY%" requires the NVIDIA profile.
  exit /b 2
)

rem The path of the title: asked for once, kept between runs.
set "GAME_FILE=%LOCALAPPDATA%\victory\game-path.txt"
if defined CHOOSE del /q "%GAME_FILE%" >nul 2>&1
set "GAME_EXE="
if exist "%GAME_FILE%" for /f "usebackq delims=" %%I in ("%GAME_FILE%") do if not defined GAME_EXE set "GAME_EXE=%%I"
if defined GAME_EXE if not exist "%GAME_EXE%" set "GAME_EXE="
if not defined GAME_EXE if not defined VICTORY_DRY_RUN call :choose

set "RUNNER=%CD%\target\%BUILD%\victory.exe"
set "LAUNCHER=%CD%\scripts\play-nvidia.ps1"
if "%PROFILE%"=="amd" set "LAUNCHER=%CD%\scripts\play.ps1"

if defined VICTORY_DRY_RUN (
  echo [dry run] powershell -NoProfile -ExecutionPolicy Bypass -File "%LAUNCHER%" -Exe "%GAME_EXE%" -Build %BUILD%%ARGS%
  exit /b 0
)
if not exist "%LAUNCHER%" (
  echo No "%LAUNCHER%".
  echo This does not look like the victory checkout.
  set "CODE=2"
  goto :stop
)
echo victory : "%RUNNER%"  ^(%BUILD%^)
echo title   : "%GAME_EXE%"
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%LAUNCHER%" -Exe "%GAME_EXE%" -Build %BUILD%%ARGS%
set "CODE=%ERRORLEVEL%"
echo.
echo victory ended with exit code %CODE%.
goto :stop

:choose
rem A file dialog, then typed input when it is cancelled (or PowerShell is
rem not there). Both write the path down in the console's own encoding, the
rem one this script reads it back with.
echo.
echo Choose the game's exe path.
for /f "usebackq delims=" %%I in (`powershell -NoProfile -ExecutionPolicy Bypass -Command "Add-Type -AssemblyName System.Windows.Forms; $f = Join-Path $env:LOCALAPPDATA 'victory\game-path.txt'; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Title = 'Choose the game''s exe path'; $d.Filter = 'Game executable (*.exe)|*.exe|All files (*.*)|*.*'; $d.CheckFileExists = $true; if ($d.ShowDialog() -ne 'OK') { exit 1 }; $e = $d.FileName; if (-not $e) { exit 1 }; New-Item -ItemType Directory -Force -Path (Split-Path $f) | Out-Null; [IO.File]::WriteAllBytes($f, [Console]::OutputEncoding.GetBytes($e)); $e"`) do if not defined GAME_EXE set "GAME_EXE=%%I"
if defined GAME_EXE (
  echo Kept as "%GAME_FILE%".
  exit /b 0
)
echo No .exe chosen.
:choose_typed
set "GAME_EXE="
set /p "GAME_EXE=Paste the path of the title's .exe (or drag it into this window), then Enter: "
if not defined GAME_EXE goto :choose_typed
set "GAME_EXE=%GAME_EXE:"=%"
if not exist "%GAME_EXE%" (
  echo "%GAME_EXE%" is not there.
  goto :choose_typed
)
if not exist "%LOCALAPPDATA%\victory" mkdir "%LOCALAPPDATA%\victory" >nul 2>&1
<nul set /p "=%GAME_EXE%" > "%GAME_FILE%"
echo Kept as "%GAME_FILE%".
exit /b 0

:stop
rem VICTORY_NO_PAUSE=1 skips the wait (shortcuts, scripts).
if not defined VICTORY_NO_PAUSE pause
exit /b %CODE%