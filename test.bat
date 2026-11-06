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
powershell -NoProfile -Command "if (-not [IO.File]::Exists('Cargo.lock')) { exit 0 }; $q = [char]34; $toml = [IO.File]::ReadAllText('Cargo.toml'); $lock = [IO.File]::ReadAllText('Cargo.lock'); $want = [regex]::Match($toml, '(?ms)^\[package\]\s*$.*?^version\s*=\s*' + $q + '([^' + $q + ']+)' + $q).Groups[1].Value; $entry = [regex]::Match($lock, '(?m)^name = ' + $q + 'GameToaster' + $q + '\r?\nversion = ' + $q + '([^' + $q + ']+)' + $q).Groups[1]; if ($want -and $entry.Success -and $entry.Value -ne $want) { [IO.File]::WriteAllText('Cargo.lock', $lock.Remove($entry.Index, $entry.Length).Insert($entry.Index, $want)); 'Cargo.lock: GameToaster ' + $entry.Value + ' -> ' + $want + ', the version Cargo.toml sets' }"
if errorlevel 1 exit /b 1
cargo test --locked --all-targets --target-dir build\cargo-target
if errorlevel 1 exit /b 1
echo GameToaster tests passed
exit /b 0
