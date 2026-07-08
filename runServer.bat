@echo off
setlocal

set "RUNNER_DIR=%~dp0..\PumpkinRunner"
set "SERVER_EXE=%RUNNER_DIR%\target\release\pumpkin.exe"

if not exist "%SERVER_EXE%" (
    echo Release server executable not found:
    echo %SERVER_EXE%
    echo Run build.bat first.
    exit /b 1
)

pushd "%RUNNER_DIR%" || exit /b 1
"%SERVER_EXE%"
set "RUN_EXIT=%ERRORLEVEL%"
popd

pause
endlocal & exit /b %RUN_EXIT%
