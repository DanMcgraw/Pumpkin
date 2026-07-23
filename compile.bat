@echo off
setlocal

set "ROOT_DIR=%~dp0"
set "PLUGIN_DIR=%ROOT_DIR%..\Cabbage"
set "RUNNER_DIR=%ROOT_DIR%..\PumpkinRunner"
set "RUNNER_PLUGIN_DIR=%RUNNER_DIR%\plugins"
set "CARGO_TARGET_DIR=%RUNNER_DIR%\target"

if not exist "%PLUGIN_DIR%\Cargo.toml" (
    echo Cabbage Cargo.toml not found:
    echo %PLUGIN_DIR%\Cargo.toml
    exit /b 1
)

if not exist "%RUNNER_PLUGIN_DIR%" (
    mkdir "%RUNNER_PLUGIN_DIR%"
    if errorlevel 1 exit /b 1
)

pushd "%PLUGIN_DIR%" || exit /b 1

cargo +stable build
if errorlevel 1 (
    popd
    exit /b 1
)

set "PLUGIN_SOURCE=%CARGO_TARGET_DIR%\debug\cabbage.dll"
if not exist "%PLUGIN_SOURCE%" (
    set "PLUGIN_SOURCE=%CARGO_TARGET_DIR%\debug\Cabbage.dll"
)
set "PLUGIN_DEST=%RUNNER_PLUGIN_DIR%\cabbage.dll"

if not exist "%PLUGIN_SOURCE%" (
    echo Plugin DLL not found:
    echo %PLUGIN_SOURCE%
    popd
    exit /b 1
)

copy /Y "%PLUGIN_SOURCE%" "%PLUGIN_DEST%"
if errorlevel 1 (
    popd
    exit /b 1
)

popd
echo Copied cabbage.dll to %PLUGIN_DEST%
endlocal
