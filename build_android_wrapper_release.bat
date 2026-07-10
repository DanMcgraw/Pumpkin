@echo off
setlocal enabledelayedexpansion

set "ROOT_DIR=%~dp0"
set "WRAPPER_DIR=%ROOT_DIR%android-wrapper"
set "CABBAGE_DIR=%ROOT_DIR%..\Cabbage"
set "JNI_DIR=%WRAPPER_DIR%\app\src\main\jniLibs\arm64-v8a"
set "CARGO_TARGET_DIR=%WRAPPER_DIR%\target\rust"

if not defined ANDROID_NDK_ROOT (
    if defined ANDROID_NDK_HOME (
        set "ANDROID_NDK_ROOT=%ANDROID_NDK_HOME%"
    )
)

if not defined ANDROID_NDK_ROOT (
    echo ERROR: ANDROID_NDK_ROOT is not set.
    exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
    echo ERROR: cargo is not on PATH.
    exit /b 1
)

where cargo-ndk >nul 2>nul
if errorlevel 1 (
    echo ERROR: cargo-ndk is not on PATH.
    exit /b 1
)

if not exist "%JNI_DIR%" (
    mkdir "%JNI_DIR%"
    if errorlevel 1 exit /b 1
)

echo Building Pumpkin Android ARM64 release executable...
pushd "%ROOT_DIR%" || exit /b 1
cargo ndk -t arm64-v8a build -p pumpkin --bin pumpkin --release
set "RUST_STATUS=%ERRORLEVEL%"
popd
if not "%RUST_STATUS%"=="0" (
    echo Rust Android build failed.
    exit /b %RUST_STATUS%
)

set "PUMPKIN_BIN=%CARGO_TARGET_DIR%\aarch64-linux-android\release\pumpkin"
if not exist "%PUMPKIN_BIN%" (
    echo ERROR: Android Pumpkin executable not found:
    echo %PUMPKIN_BIN%
    exit /b 1
)

copy /Y "%PUMPKIN_BIN%" "%JNI_DIR%\libpumpkin_exec.so" >nul
if errorlevel 1 (
    echo Failed to copy Pumpkin executable into Android wrapper.
    exit /b 1
)

if exist "%CABBAGE_DIR%\Cargo.toml" (
    echo Building Cabbage Android ARM64 release plugin...
    pushd "%CABBAGE_DIR%" || exit /b 1
    cargo ndk -t arm64-v8a build --release
    set "CABBAGE_STATUS=%ERRORLEVEL%"
    popd
    if not "!CABBAGE_STATUS!"=="0" (
        echo Cabbage Android build failed.
        exit /b !CABBAGE_STATUS!
    )

    set "CABBAGE_BIN=%CARGO_TARGET_DIR%\aarch64-linux-android\release\libcabbage.so"
    if not exist "!CABBAGE_BIN!" (
        echo ERROR: Android Cabbage plugin not found:
        echo !CABBAGE_BIN!
        exit /b 1
    )

    copy /Y "!CABBAGE_BIN!" "%JNI_DIR%\libcabbage.so" >nul
    if errorlevel 1 (
        echo Failed to copy Cabbage plugin into Android wrapper.
        exit /b 1
    )
) else (
    echo Cabbage plugin directory not found; skipping Cabbage packaging.
)

echo Building Android wrapper release APK...
pushd "%WRAPPER_DIR%" || exit /b 1
call gradlew.bat :app:assembleRelease
set "GRADLE_STATUS=%ERRORLEVEL%"
popd
if not "%GRADLE_STATUS%"=="0" (
    echo Gradle build failed.
    exit /b %GRADLE_STATUS%
)

echo Android wrapper APK built:
echo %WRAPPER_DIR%\app\build\outputs\apk\release\app-release.apk
endlocal
