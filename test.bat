@echo off
setlocal
cd /d "%~dp0"
set "RUSTFLAGS=-Dwarnings -C target-feature=+crt-static"
cargo test --locked --all-targets --target-dir build\cargo-target
if errorlevel 1 exit /b 1
echo GameToaster tests passed
exit /b 0
