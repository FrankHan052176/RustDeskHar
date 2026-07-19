# `rustdesk-ohrs`

HarmonyOS native HAR bridge for the RustDesk migration.

## Status

- RustDesk upstream snapshot is vendored at `third_party/rustdesk`.
- `libs/hbb_common` is now also vendored under the RustDesk snapshot.
- The bridge contract now mirrors RustDesk's Flutter session layer.
- The exported NAPI methods now call real RustDesk session entry points where possible.
- Polling APIs now exist for Harmony session events and RGBA frames.
- Current remaining work is wiring those polling APIs into ArkTS UI/rendering.

## Build

```shell
ohrs build --release -a aarch
ohrs artifact
```

## Current Exports

```ts
nativeVersion(): string
healthcheck(): string
backendSummary(): string
connectionFlowManifest(): string
compileBlockersManifest(): string
sessionApiManifest(): string
normalizePeerTarget(peerTarget: string, forceRelay: boolean): string
sessionAdd(sessionId: string, peerTarget: string, optionsJson: string): string
sessionStart(sessionId: string): string
sessionLogin(sessionId: string, loginJson: string): string
sessionSend2Fa(sessionId: string, code: string, trustThisDevice: boolean): string
sessionReconnect(sessionId: string, forceRelay: boolean): string
sessionClose(sessionId: string): string
sessionSendPointer(sessionId: string, pointerJson: string): string
sessionInputKey(sessionId: string, keyJson: string): string
sessionInputString(sessionId: string, value: string): string
sessionSendChat(sessionId: string, text: string): string
sessionPollEvents(sessionId: string, limit: number): string
sessionGetRgbaFrameInfo(sessionId: string, display: number): string
sessionTakeRgbaFrame(sessionId: string, display: number): Uint8Array
sessionSwitchDisplay(sessionId: string, displaysJson: string): string
sessionGet(sessionId: string): string
sessionList(): string
```

Most session methods now return JSON action results plus bridge session snapshots while forwarding into real RustDesk session APIs. Session events and RGBA frames can now be polled from NAPI; the main unfinished work is consuming them from ArkTS.
