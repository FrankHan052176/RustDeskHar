# OHOS RustDesk Patch Audit

This note records the intended patch boundary for
`third_party/rustdesk` after resetting local `master` to `origin/master`.
It is here to prevent future context resets from re-importing the noisy
`main` snapshot wholesale.

## Goal

Keep `third_party/rustdesk` as close as possible to upstream `master`,
while retaining only the changes required to build and expose the
HarmonyOS native HAR control-side surface.

Do not modify files under `flutter/`.

## Retained Patch Areas

- OHOS target cfg fixes:
  - Treat `target_env = "ohos"` as a mobile/client target.
  - Exclude desktop Linux-only code from OHOS, because Rust reports
    `target_os = "linux"` for the OHOS target.
- HAR/headless session exports:
  - Start and poll a headless session without a Flutter event stream.
  - Queue JSON UI events and RGBA notifications for ArkTS polling.
  - Expose render stats and XComponent surface lookup hooks.
- Remote cursor export:
  - Allow ArkTS to enable or disable remote cursor visibility through
    the session option path.
- OHOS stubs:
  - Replace unsupported desktop IPC, LAN, clipboard, and CM modules with
    narrow no-op OHOS modules where the mobile control-side build needs
    the symbols.
- OHOS video decode path in `libs/scrap`:
  - Wire OHOS AVCodec-backed decode modules.
  - Skip desktop/webm/hwcodec build dependencies on OHOS.
- `libs/hbb_common` OHOS adaptation:
  - Current upstream tracks this as a submodule, but the HAR build needs
    OHOS cfg, TLS, proxy/websocket, DNS, config, verifier, and mobile
    storage changes inside `hbb_common`.
  - The current local state vendors the adapted source directory. A more
    upstream-friendly future cleanup is to move these changes into a
    dedicated `hbb_common` fork/submodule and point `.gitmodules` at it.
- Required local vendor patches:
  - `vendor/libsodium-sys` is patched so the bundled libsodium C build can
    cross-compile for `aarch64-unknown-linux-ohos`.
  - `vendor/rdev` is patched only to keep Linux/X11 desktop code out of the
    OHOS build while preserving shared key conversion symbols.
  - `vendor/magnum-opus` is present because Cargo still resolves the patched
    package on non-OHOS targets. The RustDesk crate excludes audio/opus use on
    OHOS, so this is not part of the HarmonyOS runtime surface.

## Rejected From `main`

- Flutter UI/assets/application changes.
- CI and metadata churn unrelated to the HAR.
- Desktop-only dependency pinning that does not affect OHOS HAR builds.
- Generated build outputs and ad hoc local artifacts. Vendor source is kept
  only for the dependency patches listed above.

## Verification

Run from `rustdesk_native_har`:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-har.ps1
```

The expected successful end state is:

- `ohrs build --release -a aarch` completes.
- `dist\arm64-v8a\librustdesk_native_har.so` is refreshed.
- `ohrs artifact` completes.
- `package.har` is produced.
