@echo off
setlocal EnableExtensions EnableDelayedExpansion
title LessAI Launcher

cd /d "%~dp0"

echo ========================================
echo LessAI Launcher
echo ========================================
echo(

call "scripts\lessai-windows-common.bat" require_tools
if errorlevel 1 (
  echo(
  pause
  exit /b 1
)

call "scripts\lessai-windows-common.bat" prepare_node_env

call "scripts\lessai-windows-common.bat" ensure_deps
set "EXIT_CODE=!ERRORLEVEL!"
if not "%EXIT_CODE%"=="0" (
  echo(
  echo [ERROR] Environment check failed with exit code %EXIT_CODE%.
  pause
  exit /b %EXIT_CODE%
)

echo [INFO] Starting LessAI in dev mode...
echo [INFO] First launch may take a while because Rust will compile.
echo [INFO] Close the app window or this terminal to stop it.
echo(

call "scripts\lessai-windows-common.bat" ensure_dev_port_free 1420
set "EXIT_CODE=!ERRORLEVEL!"
if not "%EXIT_CODE%"=="0" (
  echo(
  echo [ERROR] Dev server port 1420 is not available.
  echo [HINT] Close the program using port 1420, or choose to terminate it when prompted.
  pause
  exit /b %EXIT_CODE%
)

call pnpm exec tauri dev
set "EXIT_CODE=%ERRORLEVEL%"

echo(
if not "%EXIT_CODE%"=="0" (
  echo [ERROR] LessAI exited with code %EXIT_CODE%.
) else (
  echo [INFO] LessAI exited normally.
)

pause
exit /b %EXIT_CODE%
