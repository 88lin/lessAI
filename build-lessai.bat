@echo off
setlocal EnableExtensions EnableDelayedExpansion
title LessAI Packager

cd /d "%~dp0"

echo ========================================
echo LessAI Packager
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

echo [INFO] Building LessAI (Tauri bundle)...
echo [INFO] This may take a while on first build.
echo(

set "RUST_BACKTRACE=1"
call pnpm exec tauri build
set "EXIT_CODE=%ERRORLEVEL%"

echo(
if not "%EXIT_CODE%"=="0" (
  echo [ERROR] LessAI build failed with exit code %EXIT_CODE%.
  echo [HINT] Make sure you are building in the same OS environment that installed node_modules.
  echo [HINT] If you see optional-deps native binding errors, repair install node_modules.
  echo(
  pause
  exit /b %EXIT_CODE%
)

echo [INFO] Build completed successfully.
echo(
echo [INFO] Output directory (default):
echo   %cd%\src-tauri\target\release\bundle
echo(
if exist "src-tauri\target\release\bundle" (
  echo [INFO] Bundles:
  dir /b "src-tauri\target\release\bundle"
) else (
  echo [WARN] Bundle directory not found. Tauri output path may differ on your system.
)

echo(
pause
exit /b 0
