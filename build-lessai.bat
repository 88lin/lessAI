@echo off
setlocal EnableExtensions
title LessAI Packager

cd /d "%~dp0"

echo ========================================
echo LessAI Packager
echo ========================================
echo(

where pnpm >nul 2>nul
if errorlevel 1 (
  echo [ERROR] pnpm was not found.
  echo [ERROR] Please install Node.js and pnpm first.
  echo(
  pause
  exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo [ERROR] cargo was not found.
  echo [ERROR] Please install the Rust toolchain first.
  echo(
  pause
  exit /b 1
)

if not exist "node_modules" (
  echo [INFO] Installing frontend dependencies...
  call pnpm install
  if errorlevel 1 (
    echo(
    echo [ERROR] Dependency installation failed.
    pause
    exit /b 1
  )
)

if not exist "node_modules\\.bin\\tauri.cmd" (
  echo [WARN] Tauri CLI was not found in node_modules.
  echo [INFO] Reinstalling dependencies - devDependencies included...
  call pnpm install
  if errorlevel 1 (
    echo(
    echo [ERROR] Dependency installation failed.
    pause
    exit /b 1
  )
)

echo [INFO] Building LessAI (Tauri bundle)...
echo [INFO] This may take a while on first build.
echo(

set "RUST_BACKTRACE=1"
call pnpm run tauri:build
set "EXIT_CODE=%ERRORLEVEL%"

echo(
if not "%EXIT_CODE%"=="0" (
  echo [ERROR] LessAI build failed with exit code %EXIT_CODE%.
  echo [HINT] If you see rollup optional-deps errors, delete node_modules and re-run pnpm install.
  echo [HINT] Make sure you are building in the same OS environment that installed node_modules.
  echo(
  pause
  exit /b %EXIT_CODE%
)

echo [INFO] Build completed successfully.
echo(
echo [INFO] Output directory (default):
echo   %cd%\\src-tauri\\target\\release\\bundle
echo(
if exist "src-tauri\\target\\release\\bundle" (
  echo [INFO] Bundles:
  dir /b "src-tauri\\target\\release\\bundle"
) else (
  echo [WARN] Bundle directory not found. Tauri output path may differ on your system.
)

echo(
pause
exit /b 0

