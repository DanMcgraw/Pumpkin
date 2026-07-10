# Pumpkin Android Wrapper

This is a separate Android testing pipeline for running Pumpkin on a phone.
It does not use the normal Windows `PumpkinRunner` build path.

## Build Only

```bat
build_android_wrapper_release.bat
```

The script builds the `pumpkin` binary for `arm64-v8a` with `--release`.
If `..\Cabbage` exists, it also builds Cabbage as `libcabbage.so`. Both native
artifacts are copied into the Android wrapper before assembling a release APK
signed with the debug key for local sideload testing.

## Build, Install, Launch

```bat
deploy_android_wrapper_release.bat
```

The wrapper starts Pumpkin from a foreground service. Use the app UI or the
notification Stop action to stop the server. Console commands are sent to
Pumpkin stdin, so `stop`, `list`, `op`, and other console commands use the
same path as desktop stdin. When `libcabbage.so` is packaged, the service
auto-runs `plugin load` after Pumpkin reports that the server is running.

The first launch copies `pumpkin.toml` from app assets into app-private
storage. Later launches reuse the app-private config and world data.
