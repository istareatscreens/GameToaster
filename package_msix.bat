@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0packaging\msix\package.ps1" -Describe >nul
if %errorlevel% neq 0 goto :describe_failed
call "%~dp0test.bat"
if %errorlevel% neq 0 goto :test_failed
call "%~dp0build.bat"
if %errorlevel% neq 0 goto :build_failed
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0packaging\msix\package.ps1"
if %errorlevel% neq 0 goto :package_failed
echo To try the package with Developer Mode on: powershell -NoProfile -Command "Add-AppxPackage -Register '%~dp0build\msix\layout\AppxManifest.xml'"
call :hold
exit /b 0
:describe_failed
echo package_msix.bat: the identity and version check failed; see the line above.
call :hold
exit /b 1
:test_failed
echo package_msix.bat: test.bat failed; see above.
call :hold
exit /b 1
:build_failed
echo package_msix.bat: build.bat failed; see above.
call :hold
exit /b 1
:package_failed
echo package_msix.bat: packaging failed; see the line above.
call :hold
exit /b 1
:hold
set "cmdline=%cmdcmdline:"=%"
if not "%cmdline:/c=%"=="%cmdline%" pause
exit /b 0
