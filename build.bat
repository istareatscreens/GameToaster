@echo off
setlocal
cd /d "%~dp0"
if exist build_output\GameToaster.exe (
  2>nul (>>build_output\GameToaster.exe (call )) || (
    echo build_output\GameToaster.exe is in use. Close GameToaster and run again.
    exit /b 1
  )
)
for /f "delims=" %%U in ('powershell -NoProfile -Command "[char]31"') do set "US=%%U"
if not defined US (echo The build needs PowerShell to compose the compiler flags.& exit /b 1)
if not defined CARGO_HOME set "CARGO_HOME=%USERPROFILE%\.cargo"
for /f "delims=" %%S in ('rustc --print sysroot') do set "RUST_SYSROOT=%%S"
if not defined RUST_SYSROOT (echo rustc was not found; install the Rust toolchain from https://rustup.rs& exit /b 1)
set "CARGO_ENCODED_RUSTFLAGS=-Dwarnings%US%-C%US%target-feature=+crt-static%US%--remap-path-prefix=%CD%=/gametoaster%US%--remap-path-prefix=%CARGO_HOME%=/cargo%US%--remap-path-prefix=%RUST_SYSROOT%=/rustc-sysroot"
set "RUSTFLAGS="
cargo build --locked --release --target-dir build\cargo-target
if errorlevel 1 exit /b 1
if not exist build_output mkdir build_output
copy /y build\cargo-target\release\GameToaster.exe build_output\GameToaster.exe >nul
if errorlevel 1 exit /b 1
echo GameToaster build complete: %CD%\build_output
exit /b 0
