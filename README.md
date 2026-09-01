# Android Terminal

A Rust GUI for debugging Android devices and emulators on macOS.

## Prerequisites

- **Rust** 1.88+ (see `rust-toolchain.toml`)
- **Android SDK platform-tools** with `adb` on your `PATH`

Install platform-tools via [Android Studio](https://developer.android.com/studio) or the [SDK command-line tools](https://developer.android.com/studio#command-tools), then verify:

```bash
adb version
```

## Run

```bash
cargo run -p android-terminal
```

An Android emulator or USB-connected device must be running and authorized (`adb devices` should list it as `device`).

## Project layout

```
crates/
  adb-client/       # adb command wrappers and parsing
  android-terminal/ # GUI application
```
