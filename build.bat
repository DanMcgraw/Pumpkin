@echo off
setlocal

pushd "%~dp0" || exit /b 1
cargo build --release -p pumpkin --bin pumpkin %*
set "BUILD_EXIT=%ERRORLEVEL%"
popd

endlocal & exit /b %BUILD_EXIT%
