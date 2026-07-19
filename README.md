# RustDeskHar

HarmonyOS Native HAR bridge for the RustDesk OHOS controller port.

Public mirror: `https://atomgit.com/FrankHan2004/RustDeskHar`

## Layout

- `src/`: NAPI bridge entry points exported by `librustdesk_native_har.so`.
- `package/`: HAR packaging metadata consumed by `ohrs artifact`.
- `third_party/rustdesk/`: RustDesk OHOS fork submodule. Its upstream remote is `git@github.com:FrankHan052176/rustdesk4ohos.git`.
- `scripts/`: reusable local and CI build entry points.

`rust-webm` is intentionally not vendored by this HAR repository. Cargo resolves it from the RustDesk dependency graph and lockfile, including its upstream nested `libwebm` submodule.

## Clone

```bash
git clone --recurse-submodules git@github.com:FrankHan052176/RustDeskHar.git
cd RustDeskHar
git submodule update --init --recursive
```

If the repository was cloned without submodules, run:

```bash
git submodule update --init --recursive
```

## Build

Install Rust and `ohrs`, then point `OHOS_NDK_HOME` at the HarmonyOS/OpenHarmony SDK `openharmony` directory:

```bash
cargo install ohrs
export OHOS_NDK_HOME="$HOME/Huawei/Sdk/default/openharmony"
scripts/build-har.sh
```

The script exports the aarch64 OHOS toolchain variables and runs:

```bash
ohrs build --release -a aarch
ohrs artifact
```

The final HAR artifact is generated at `package.har`.

### Windows

Windows builds use the MSVC Rust host toolchain and Visual Studio C++ Build Tools for host-side build scripts. The target-side compiler is still the HarmonyOS/OpenHarmony NDK clang for `aarch64-unknown-linux-ohos`.

Required tools:

- Rust stable MSVC toolchain.
- Visual Studio Build Tools with the C++ toolchain workload.
- DevEco Studio or command line HarmonyOS/OpenHarmony SDK.
- Git for Windows, plus a POSIX `sh.exe` and `mingw32-make.exe` available to crates that build autotools dependencies.
- `ohrs` installed in `PATH`.

Run:

```powershell
cargo install ohrs --locked
$env:OHOS_NDK_HOME = 'C:\Program Files\Huawei\DevEco Studio\sdk\default\openharmony'
powershell -ExecutionPolicy Bypass -File .\scripts\build-har.ps1
```

The PowerShell script imports `VsDevCmd.bat` automatically when needed, resolves the OHOS NDK, converts paths with spaces to shell-safe short paths for autotools/libtool, and runs:

```powershell
ohrs build --release -a aarch
ohrs artifact
```

`libsodium-sys` is intentionally single-threaded on `windows + linux-ohos` in the RustDesk fork because Git-for-Windows/MSYS libtool can race when producing intermediate archives. A clean Windows build can therefore spend a long quiet period in `libsodium-sys`; this is expected as long as object timestamps keep advancing.

Only `arm64-v8a` is packaged. The build scripts clear stale `x86`, `x86_64`, and `armeabi-v7a` directories from both `dist` and `package/libs` before `ohrs artifact`, and they refuse to package a HAR if the arm64 native library was not refreshed by the current build.

### Linux and macOS

Linux and macOS builds use the same target model: host tools run natively, while C/C++ target code is compiled by the HarmonyOS/OpenHarmony NDK toolchain. Install Rust, `ohrs`, Git, CMake/build essentials, and the HarmonyOS/OpenHarmony SDK, then run:

```bash
export OHOS_NDK_HOME="$HOME/Huawei/Sdk/default/openharmony"
scripts/build-har.sh
```

## CI

The GitHub Actions workflow lives at `.github/workflows/build-har.yml`.

HarmonyOS SDK redistribution is not assumed, so the workflow targets a Linux self-hosted runner. Configure the runner with:

- Rust toolchain access.
- `cargo` and network access for `cargo install ohrs`.
- HarmonyOS/OpenHarmony SDK installed locally.
- `OHOS_NDK_HOME` exported to the SDK `openharmony` directory.
- SSH access to `git@github.com:FrankHan052176/rustdesk4ohos.git` for recursive submodule checkout.
- Network access to Cargo registries, `https://github.com/rustdesk-org/rust-webm.git`, and `https://chromium.googlesource.com/webm/libwebm`.

If the runner cannot access Chromium Git directly, set `LIBWEBM_GIT_MIRROR` to a compatible mirror URL such as:

```bash
https://github.com/webmproject/libwebm.git
```

The RustDesk `rust-webm` git dependency still records the original libwebm submodule URL; this variable only rewrites the checkout URL on the CI runner.

## Updating RustDesk

RustDesk changes belong in the submodule, not in the HAR parent repository:

```bash
cd third_party/rustdesk
git remote -v
git remote set-url origin git@github.com:FrankHan052176/rustdesk4ohos.git
git fetch origin
git checkout main
git merge origin/main
```

After updating or committing RustDesk changes, return to the HAR repository and commit the submodule pointer:

```bash
cd ../..
git status
git add third_party/rustdesk
git commit -m "Update RustDesk submodule"
```

This keeps upstream RustDesk merging and PR work inside `rustdesk4ohos`, while `RustDeskHar` only maintains the reusable HarmonyOS HAR bridge and build pipeline.
