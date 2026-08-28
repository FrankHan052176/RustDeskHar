import native from 'librustdesk_native_har.so';

export function backendSummary(): string {
  return native.backendSummary();
}

export function buildMarker(): string {
  return native.buildMarker();
}

export function healthcheck(): string {
  return native.healthcheck();
}

export function controlledServerStart(configJson: string): string { return native.controlledServerStart(configJson); }
export function controlledServerStop(): string { return native.controlledServerStop(); }
export function controlledServerGetStatus(): string { return native.controlledServerGetStatus(); }
export function controlledIncomingPoll(limit: number): string { return native.controlledIncomingPoll(limit); }
export function controlledIncomingResolve(requestId: string, accepted: boolean): string { return native.controlledIncomingResolve(requestId, accepted); }
export function controlledIncomingSetPermission(requestId: string, permission: string, enabled: boolean): string { return native.controlledIncomingSetPermission(requestId, permission, enabled); }
export function controlledPasswordGet(): string { return native.controlledPasswordGet(); }
export function controlledPasswordRefresh(): string { return native.controlledPasswordRefresh(); }
export function controlledPasswordSetPermanent(password: string): string { return native.controlledPasswordSetPermanent(password); }
export function controlledPasswordSetVerificationMethod(method: string): string {
  return native.controlledPasswordSetVerificationMethod(method);
}
export function controlledSettingsGet(): string { return native.controlledSettingsGet(); }
export function controlledSettingSet(key: string, value: string): string {
  return native.controlledSettingSet(key, value);
}
export function controlledTwoFactorGet(): string { return native.controlledTwoFactorGet(); }
export function controlledTwoFactorBegin(): string { return native.controlledTwoFactorBegin(); }
export function controlledTwoFactorVerify(code: string): string { return native.controlledTwoFactorVerify(code); }
export function controlledTwoFactorDisable(): string { return native.controlledTwoFactorDisable(); }
export function controlledTwoFactorRemoveTrustedDevices(hwidsJson: string): string {
  return native.controlledTwoFactorRemoveTrustedDevices(hwidsJson);
}
export function controlledTwoFactorClearTrustedDevices(): string {
  return native.controlledTwoFactorClearTrustedDevices();
}
export function controlledScreenConfigure(configJson: string): string { return native.controlledScreenConfigure(configJson); }
export function controlledScreenCaptureStart(configJson: string): string { return native.controlledScreenCaptureStart(configJson); }
export function controlledScreenCaptureStop(): string { return native.controlledScreenCaptureStop(); }
export function controlledScreenCaptureGetStatus(): string { return native.controlledScreenCaptureGetStatus(); }

export function inputInterceptorPollEvents(limit: number): string {
  return native.inputInterceptorPollEvents(limit);
}

export function inputInterceptorStart(): string {
  return native.inputInterceptorStart();
}

export function inputInterceptorStop(): string {
  return native.inputInterceptorStop();
}

export function nativeVersion(): string {
  return native.nativeVersion();
}

export function runtimeGetApiServer(): string {
  return native.runtimeGetApiServer();
}

export function runtimeGetAccountState(): string {
  return native.runtimeGetAccountState();
}

export function runtimeGetAccountLoginOptions(): string {
  return native.runtimeGetAccountLoginOptions();
}

export function runtimeStartAccountLoginOptions(): string {
  return native.runtimeStartAccountLoginOptions();
}

export function runtimePollAccountLoginOptions(): string {
  return native.runtimePollAccountLoginOptions();
}

export function runtimeStartAccountLogin(payloadJson: string): string {
  return native.runtimeStartAccountLogin(payloadJson);
}

export function runtimeStartAccountOidc(provider: string): string {
  return native.runtimeStartAccountOidc(provider);
}

export function runtimePollAccountOidc(): string {
  return native.runtimePollAccountOidc();
}

export function runtimeCancelAccountOidc(): string {
  return native.runtimeCancelAccountOidc();
}

export function runtimeStartAccountVerification(payloadJson: string): string {
  return native.runtimeStartAccountVerification(payloadJson);
}

export function runtimePollAccountAction(): string {
  return native.runtimePollAccountAction();
}

export function runtimeAccountLogout(): string {
  return native.runtimeAccountLogout();
}

export function runtimeCancelAccountChallenge(): string {
  return native.runtimeCancelAccountChallenge();
}

export function runtimeStartAddressBookSync(): string {
  return native.runtimeStartAddressBookSync();
}

export function runtimePollAddressBookSync(): string {
  return native.runtimePollAddressBookSync();
}

export function runtimeGetDecoderCapabilities(): string {
  return native.runtimeGetDecoderCapabilities();
}

export function runtimeGetServerConfig(): string {
  return native.runtimeGetServerConfig();
}

export function runtimeInit(appDir: string, deviceName: string): string {
  return native.runtimeInit(appDir, deviceName);
}

export function runtimeListAddressBookPeers(): string {
  return native.runtimeListAddressBookPeers();
}

export function runtimeListFavorites(): string {
  return native.runtimeListFavorites();
}

export function runtimeListLanPeers(): string {
  return native.runtimeListLanPeers();
}

export function runtimeListLocalDir(path: string): string {
  return native.runtimeListLocalDir(path);
}

export function runtimeListRecentPeers(): string {
  return native.runtimeListRecentPeers();
}

export function runtimeQueryPeerOnlineStates(peerIdsJson: string): Promise<string> {
  return native.runtimeQueryPeerOnlineStates(peerIdsJson);
}

export function runtimeProbePeerDirectStates(targetsJson: string): Promise<string> {
  return native.runtimeProbePeerDirectStates(targetsJson);
}

export function runtimeRemoveRecentPeer(peerId: string): string {
  return native.runtimeRemoveRecentPeer(peerId);
}

export function runtimeScanLanPeers(): Promise<string> {
  return native.runtimeScanLanPeers();
}

export function runtimeSetFavorite(peerId: string, favorite: boolean): string {
  return native.runtimeSetFavorite(peerId, favorite);
}

export function runtimeSetServerConfig(configJson: string): string {
  return native.runtimeSetServerConfig(configJson);
}

export function runtimeTestServerConfig(configJson: string): Promise<string> {
  return native.runtimeTestServerConfig(configJson);
}

export function sessionAdd(sessionId: string, peerTarget: string, optionsJson: string): string {
  return native.sessionAdd(sessionId, peerTarget, optionsJson);
}

export function sessionAlternativeCodecs(sessionId: string): string {
  return native.sessionAlternativeCodecs(sessionId);
}

export function sessionBindSurface(sessionId: string, display: number, surfaceId: string): string {
  return native.sessionBindSurface(sessionId, display, surfaceId);
}

export function sessionCancelJob(sessionId: string, actId: number): string {
  return native.sessionCancelJob(sessionId, actId);
}

export function sessionChangeResolution(sessionId: string, display: number, width: number, height: number): string {
  return native.sessionChangeResolution(sessionId, display, width, height);
}

export function sessionClose(sessionId: string): string {
  return native.sessionClose(sessionId);
}

export function sessionEnterOrLeave(sessionId: string, enter: boolean): string {
  return native.sessionEnterOrLeave(sessionId, enter);
}

export function sessionGetConnToken(sessionId: string): string | null {
  return native.sessionGetConnToken(sessionId);
}

export function sessionGetEnableTrustedDevices(sessionId: string): boolean {
  return native.sessionGetEnableTrustedDevices(sessionId);
}

export function sessionGetImageQuality(sessionId: string): string | null {
  return native.sessionGetImageQuality(sessionId);
}

export function sessionGetRenderStats(sessionId: string, display: number): string {
  return native.sessionGetRenderStats(sessionId, display);
}

export function sessionGetRemoteAudioState(sessionId: string): string {
  return native.sessionGetRemoteAudioState(sessionId);
}

export function sessionGetRgbaSize(sessionId: string, display: number): number {
  return native.sessionGetRgbaSize(sessionId, display);
}

export function sessionGetToggleOption(sessionId: string, arg: string): boolean {
  return native.sessionGetToggleOption(sessionId, arg);
}

export function sessionHandleFlutterKeyEvent(sessionId: string, keyJson: string): string {
  return native.sessionHandleFlutterKeyEvent(sessionId, keyJson);
}

export function sessionInputKey(sessionId: string, keyJson: string): string {
  return native.sessionInputKey(sessionId, keyJson);
}

export function sessionInputString(sessionId: string, value: string): string {
  return native.sessionInputString(sessionId, value);
}

export function sessionLogin(sessionId: string, loginJson: string): string {
  return native.sessionLogin(sessionId, loginJson);
}

export function sessionNextRgba(sessionId: string, display: number): void {
  native.sessionNextRgba(sessionId, display);
}

export function sessionOnWaitingForImageDialogShow(sessionId: string): string {
  return native.sessionOnWaitingForImageDialogShow(sessionId);
}

export function sessionPeerOption(sessionId: string, name: string, value: string): void {
  native.sessionPeerOption(sessionId, name, value);
}

export function sessionPollEvents(sessionId: string, limit: number): string {
  return native.sessionPollEvents(sessionId, limit);
}

export function sessionReadRemoteDir(sessionId: string, path: string, includeHidden: boolean): string {
  return native.sessionReadRemoteDir(sessionId, path, includeHidden);
}

export function sessionReconnect(sessionId: string, forceRelay: boolean): string {
  return native.sessionReconnect(sessionId, forceRelay);
}

export function sessionRefresh(sessionId: string, display: number): string {
  return native.sessionRefresh(sessionId, display);
}

export function sessionSend2Fa(sessionId: string, code: string, trustThisDevice: boolean): string {
  return native.sessionSend2Fa(sessionId, code, trustThisDevice);
}

export function sessionSendChat(sessionId: string, text: string): string {
  return native.sessionSendChat(sessionId, text);
}

export function sessionSendClipboard(sessionId: string, content: string): string {
  return native.sessionSendClipboard(sessionId, content);
}

export function sessionSendFiles(sessionId: string, actId: number, path: string, to: string, fileNum: number, includeHidden: boolean, isRemote: boolean, isDir: boolean): string {
  return native.sessionSendFiles(sessionId, actId, path, to, fileNum, includeHidden, isRemote, isDir);
}

export function sessionSendMouse(sessionId: string, mouseJson: string): string {
  return native.sessionSendMouse(sessionId, mouseJson);
}

export function sessionSendNote(sessionId: string, note: string): void {
  native.sessionSendNote(sessionId, note);
}

export function sessionSendPointer(sessionId: string, pointerJson: string): string {
  return native.sessionSendPointer(sessionId, pointerJson);
}

export function sessionSetCodecPreference(sessionId: string, codec: string): string {
  return native.sessionSetCodecPreference(sessionId, codec);
}

export function sessionSetCommon(sessionId: string, key: string, value: string): void {
  native.sessionSetCommon(sessionId, key, value);
}

export function sessionSetConfirmOverrideFile(sessionId: string, actId: number, fileNum: number, needOverride: boolean, remember: boolean, isUpload: boolean): string {
  return native.sessionSetConfirmOverrideFile(sessionId, actId, fileNum, needOverride, remember, isUpload);
}

export function sessionSetCustomFps(sessionId: string, fps: number): string {
  return native.sessionSetCustomFps(sessionId, fps);
}

export function sessionSetCustomImageQuality(sessionId: string, value: number): void {
  native.sessionSetCustomImageQuality(sessionId, value);
}

export function sessionSetImageQuality(sessionId: string, value: string): string {
  return native.sessionSetImageQuality(sessionId, value);
}

export function sessionSetShowRemoteCursor(sessionId: string, enabled: boolean): string {
  return native.sessionSetShowRemoteCursor(sessionId, enabled);
}

export function sessionSetSize(sessionId: string, display: number, width: number, height: number): string {
  return native.sessionSetSize(sessionId, display, width, height);
}

export function sessionStart(sessionId: string): string {
  return native.sessionStart(sessionId);
}

export function sessionSetViewOnly(sessionId: string, enabled: boolean): string {
  return native.sessionSetViewOnly(sessionId, enabled);
}

export function sessionSwitchDisplay(sessionId: string, displaysJson: string): string {
  return native.sessionSwitchDisplay(sessionId, displaysJson);
}

export function sessionTakeRgbaFrame(sessionId: string, display: number): Uint8Array {
  return native.sessionTakeRgbaFrame(sessionId, display);
}

export function sessionToggleOption(sessionId: string, arg: string): void {
  native.sessionToggleOption(sessionId, arg);
}

export function transferNextJobId(): number {
  return native.transferNextJobId();
}
