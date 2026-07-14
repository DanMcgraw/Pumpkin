@echo off
setlocal
python "%~dp0tools\bedrock_test_runner.py" %*
set "runner_exit=%errorlevel%"
endlocal & exit /b %runner_exit%
