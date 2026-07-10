@echo off
setlocal

set "ROOT_DIR=%~dp0"
set "WRAPPER_DIR=%ROOT_DIR%android-wrapper"
set "PACKAGE_NAME=com.pumpkinmc.serverwrapper"

where adb >nul 2>nul
if errorlevel 1 (
    echo ERROR: adb is not on PATH.
    exit /b 1
)

call "%ROOT_DIR%build_android_wrapper_release.bat"
set "BUILD_STATUS=%ERRORLEVEL%"
if not "%BUILD_STATUS%"=="0" (
    exit /b %BUILD_STATUS%
)

echo Installing Android wrapper on connected device...
pushd "%WRAPPER_DIR%" || exit /b 1
call gradlew.bat :app:installRelease
set "INSTALL_STATUS=%ERRORLEVEL%"
popd
if not "%INSTALL_STATUS%"=="0" (
    echo Gradle install failed.
    exit /b %INSTALL_STATUS%
)

echo Launching Pumpkin Server wrapper...
adb shell am force-stop %PACKAGE_NAME% >nul 2>nul
adb shell am start -n %PACKAGE_NAME%/.MainActivity
if errorlevel 1 (
    echo adb launch failed.
    exit /b 1
)

echo Done.
endlocal
