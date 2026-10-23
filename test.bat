@echo off
setlocal
cd /d "%~dp0"
for /f "delims=" %%U in ('powershell -NoProfile -Command "[char]31"') do set "US=%%U"
if not defined US (echo The build needs PowerShell to compose the compiler flags.& exit /b 1)
if not defined CARGO_HOME set "CARGO_HOME=%USERPROFILE%\.cargo"
for /f "delims=" %%S in ('rustc --print sysroot') do set "RUST_SYSROOT=%%S"
if not defined RUST_SYSROOT (echo rustc was not found; install the Rust toolchain from https://rustup.rs& exit /b 1)
set "CARGO_ENCODED_RUSTFLAGS=-Dwarnings%US%-C%US%target-feature=+crt-static%US%--remap-path-prefix=%CD%=/gametoaster%US%--remap-path-prefix=%CARGO_HOME%=/cargo%US%--remap-path-prefix=%RUST_SYSROOT%=/rustc-sysroot"
set "RUSTFLAGS="
cargo test --locked --all-targets --target-dir build\cargo-target
if errorlevel 1 exit /b 1
echo GameToaster tests passed
exit /b 0
