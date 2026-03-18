@echo off
setlocal EnableExtensions
title LessAI Launcher

cd /d "%~dp0"

echo ========================================
echo LessAI Launcher
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

echo [INFO] Starting LessAI in dev mode...
echo [INFO] First launch may take a while because Rust will compile.
echo [INFO] Close the app window or this terminal to stop it.
echo(

call pnpm run tauri:dev
set "EXIT_CODE=%ERRORLEVEL%"

echo(
if not "%EXIT_CODE%"=="0" (
  echo [ERROR] LessAI exited with code %EXIT_CODE%.
) else (
  echo [INFO] LessAI exited normally.
)

pause
exit /b %EXIT_CODE%
