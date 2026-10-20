@echo off
setlocal
cd /d "%~dp0"
if exist build_output\GameToaster.exe (
  2>nul (>>build_output\GameToaster.exe (call )) || (
    echo build_output\GameToaster.exe is in use. Close GameToaster and run again.
    exit /b 1
  )
)
set "RUSTFLAGS=-Dwarnings -C target-feature=+crt-static"
cargo build --locked --release --target-dir build\cargo-target
if errorlevel 1 exit /b 1
if not exist build_output mkdir build_output
copy /y build\cargo-target\release\GameToaster.exe build_output\GameToaster.exe >nul
if errorlevel 1 exit /b 1
echo GameToaster build complete: %CD%\build_output
exit /b 0
