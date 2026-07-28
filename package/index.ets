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

export function inputInterceptorPollEvents(limit: number): string {
  return native.inputInterceptorPollEvents(limit);
}

export function inputInterceptorStart(): string {
  return native.inputInterceptorStart();
}

export function inputInterceptorStop(): string {
  return native.inputInterceptorStop();
}

export function mainAudioSupportLoopback(): boolean {
  return native.mainAudioSupportLoopback();
}

export function mainCheckHwcodec(): void {
  native.mainCheckHwcodec();
}

export function mainCheckMouseTime(): void {
  native.mainCheckMouseTime();
}

export function mainCheckSuperUserPermission(): boolean {
  return native.mainCheckSuperUserPermission();
}

export function mainClearTrustedDevices(): void {
  native.mainClearTrustedDevices();
}

export function mainCreateShortcut(id: string): void {
  native.mainCreateShortcut(id);
}

export function mainCurrentIsWayland(): boolean {
  return native.mainCurrentIsWayland();
}

export function mainDefaultPrivacyModeImpl(): string {
  return native.mainDefaultPrivacyModeImpl();
}

export function mainForgetPassword(id: string): void {
  native.mainForgetPassword(id);
}

export function mainGenerate2Fa(): string {
  return native.mainGenerate2Fa();
}

export function mainGetApiServer(): string {
  return native.mainGetApiServer();
}

export function mainGetAppName(): string {
  return native.mainGetAppName();
}

export function mainGetAsyncStatus(): string {
  return native.mainGetAsyncStatus();
}

export function mainGetBuildDate(): string {
  return native.mainGetBuildDate();
}

export function mainGetBuildinOption(key: string): string {
  return native.mainGetBuildinOption(key);
}

export function mainGetCommon(key: string): string {
  return native.mainGetCommon(key);
}

export function mainGetCommonSync(key: string): string {
  return native.mainGetCommonSync(key);
}

export function mainGetConnectStatus(): string {
  return native.mainGetConnectStatus();
}

export function mainGetDisplays(): string {
  return native.mainGetDisplays();
}

export function mainGetError(): string {
  return native.mainGetError();
}

export function mainGetFav(): Array<string> {
  return native.mainGetFav();
}

export function mainGetFingerprint(): string {
  return native.mainGetFingerprint();
}

export function mainGetHardOption(key: string): string {
  return native.mainGetHardOption(key);
}

export function mainGetHomeDir(): string {
  return native.mainGetHomeDir();
}

export function mainGetLanPeers(): string {
  return native.mainGetLanPeers();
}

export function mainGetLangs(): string {
  return native.mainGetLangs();
}

export function mainGetLastRemoteId(): string {
  return native.mainGetLastRemoteId();
}

export function mainGetLicense(): string {
  return native.mainGetLicense();
}

export function mainGetLocalOption(key: string): string {
  return native.mainGetLocalOption(key);
}

export function mainGetLoginDeviceInfo(): string {
  return native.mainGetLoginDeviceInfo();
}

export function mainGetMainDisplay(): string {
  return native.mainGetMainDisplay();
}

export function mainGetMouseTime(): number {
  return native.mainGetMouseTime();
}

export function mainGetMyId(): string {
  return native.mainGetMyId();
}

export function mainGetNewStoredPeers(): string {
  return native.mainGetNewStoredPeers();
}

export function mainGetNewVersion(): string {
  return native.mainGetNewVersion();
}

export function mainGetOption(key: string): string {
  return native.mainGetOption(key);
}

export function mainGetOptions(): string {
  return native.mainGetOptions();
}

export function mainGetPeerFlutterOptionSync(id: string, key: string): string {
  return native.mainGetPeerFlutterOptionSync(id, key);
}

export function mainGetPeerOption(id: string, key: string): string {
  return native.mainGetPeerOption(id, key);
}

export function mainGetPeerOptionSync(id: string, key: string): string {
  return native.mainGetPeerOptionSync(id, key);
}

export function mainGetPeerSync(id: string): string {
  return native.mainGetPeerSync(id);
}

export function mainGetPrinterNames(): string {
  return native.mainGetPrinterNames();
}

export function mainGetProxyStatus(): boolean {
  return native.mainGetProxyStatus();
}

export function mainGetSocks(): Array<string> {
  return native.mainGetSocks();
}

export function mainGetSoftwareUpdateUrl(): void {
  native.mainGetSoftwareUpdateUrl();
}

export function mainGetSoundInputs(): Array<string> {
  return native.mainGetSoundInputs();
}

export function mainGetTemporaryPassword(): string {
  return native.mainGetTemporaryPassword();
}

export function mainGetTrustedDevices(): string {
  return native.mainGetTrustedDevices();
}

export function mainGetUnlockPin(): string {
  return native.mainGetUnlockPin();
}

export function mainGetUseTextureRender(): boolean {
  return native.mainGetUseTextureRender();
}

export function mainGetUserDefaultOption(key: string): string {
  return native.mainGetUserDefaultOption(key);
}

export function mainGetUuid(): string {
  return native.mainGetUuid();
}

export function mainGetVersion(): string {
  return native.mainGetVersion();
}

export function mainGotoInstall(): boolean {
  return native.mainGotoInstall();
}

export function mainHandleRelayId(id: string): string {
  return native.mainHandleRelayId(id);
}

export function mainHasFileClipboard(): boolean {
  return native.mainHasFileClipboard();
}

export function mainHasGpuTextureRender(): boolean {
  return native.mainHasGpuTextureRender();
}

export function mainHasHwcodec(): boolean {
  return native.mainHasHwcodec();
}

export function mainHasValid2FaSync(): boolean {
  return native.mainHasValid2FaSync();
}

export function mainHasValidBotSync(): boolean {
  return native.mainHasValidBotSync();
}

export function mainHasVram(): boolean {
  return native.mainHasVram();
}

export function mainHideDock(): boolean {
  return native.mainHideDock();
}

export function mainInitInputSource(): void {
  native.mainInitInputSource();
}

export function mainIsCanInputMonitoring(prompt: boolean): boolean {
  return native.mainIsCanInputMonitoring(prompt);
}

export function mainIsCanScreenRecording(prompt: boolean): boolean {
  return native.mainIsCanScreenRecording(prompt);
}

export function mainIsInstalled(): boolean {
  return native.mainIsInstalled();
}

export function mainIsInstalledDaemon(prompt: boolean): boolean {
  return native.mainIsInstalledDaemon(prompt);
}

export function mainIsInstalledLowerVersion(): boolean {
  return native.mainIsInstalledLowerVersion();
}

export function mainIsLoginWayland(): boolean {
  return native.mainIsLoginWayland();
}

export function mainIsOptionFixed(key: string): boolean {
  return native.mainIsOptionFixed(key);
}

export function mainIsProcessTrusted(prompt: boolean): boolean {
  return native.mainIsProcessTrusted(prompt);
}

export function mainIsShareRdp(): boolean {
  return native.mainIsShareRdp();
}

export function mainLoadFavPeers(): void {
  native.mainLoadFavPeers();
}

export function mainLoadLanPeers(): void {
  native.mainLoadLanPeers();
}

export function mainLoadRecentPeers(): void {
  native.mainLoadRecentPeers();
}

export function mainLoadRecentPeersForAb(filter: string): string {
  return native.mainLoadRecentPeersForAb(filter);
}

export function mainMaxEncryptLen(): number {
  return native.mainMaxEncryptLen();
}

export function mainPeerExists(id: string): boolean {
  return native.mainPeerExists(id);
}

export function mainPeerHasPassword(id: string): boolean {
  return native.mainPeerHasPassword(id);
}

export function mainRemovePeer(id: string): void {
  native.mainRemovePeer(id);
}

export function mainRemoveTrustedDevices(json: string): void {
  native.mainRemoveTrustedDevices(json);
}

export function mainSetCommon(key: string, value: string): void {
  native.mainSetCommon(key, value);
}

export function mainSetLocalOption(key: string, value: string): void {
  native.mainSetLocalOption(key, value);
}

export function mainSetOption(key: string, value: string): void {
  native.mainSetOption(key, value);
}

export function mainSetOptions(json: string): void {
  native.mainSetOptions(json);
}

export function mainSetPeerAlias(id: string, alias: string): void {
  native.mainSetPeerAlias(id, alias);
}

export function mainSetPeerFlutterOptionSync(id: string, key: string, value: string): void {
  native.mainSetPeerFlutterOptionSync(id, key, value);
}

export function mainSetPeerOption(id: string, key: string, value: string): void {
  native.mainSetPeerOption(id, key, value);
}

export function mainSetPeerOptionSync(id: string, key: string, value: string): boolean {
  return native.mainSetPeerOptionSync(id, key, value);
}

export function mainSetShareRdp(enable: boolean): void {
  native.mainSetShareRdp(enable);
}

export function mainSetUnlockPin(pin: string): string {
  return native.mainSetUnlockPin(pin);
}

export function mainSetUserDefaultOption(key: string, value: string): void {
  native.mainSetUserDefaultOption(key, value);
}

export function mainStoreFav(favs: Array<string>): void {
  native.mainStoreFav(favs);
}

export function mainSupportRemoveWallpaper(): boolean {
  return native.mainSupportRemoveWallpaper();
}

export function mainSupportedHwdecodings(): string {
  return native.mainSupportedHwdecodings();
}

export function mainSupportedInputSource(): string {
  return native.mainSupportedInputSource();
}

export function mainSupportedPrivacyModeImpls(): string {
  return native.mainSupportedPrivacyModeImpls();
}

export function mainUpdateMe(): boolean {
  return native.mainUpdateMe();
}

export function mainUpdateTemporaryPassword(): void {
  native.mainUpdateTemporaryPassword();
}

export function mainVerify2Fa(code: string): boolean {
  return native.mainVerify2Fa(code);
}

export function mainVerifyBot(token: string): string {
  return native.mainVerifyBot(token);
}

export function mainWol(id: string): void {
  native.mainWol(id);
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

export function runtimeRemoveRecentPeer(peerId: string): string {
  return native.runtimeRemoveRecentPeer(peerId);
}

export function runtimeScanLanPeers(): string {
  return native.runtimeScanLanPeers();
}

export function runtimeSetFavorite(peerId: string, favorite: boolean): string {
  return native.runtimeSetFavorite(peerId, favorite);
}

export function runtimeSetServerConfig(configJson: string): string {
  return native.runtimeSetServerConfig(configJson);
}

export function runtimeTestServerConfig(configJson: string): string {
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

export function sessionGetAuditGuid(sessionId: string): string {
  return native.sessionGetAuditGuid(sessionId);
}

export function sessionGetAuditServerSync(sessionId: string, typ: string): string {
  return native.sessionGetAuditServerSync(sessionId, typ);
}

export function sessionGetCommon(sessionId: string, key: string, param: string): string | null {
  return native.sessionGetCommon(sessionId, key, param);
}

export function sessionGetCommonSync(sessionId: string, key: string, param: string): string | null {
  return native.sessionGetCommonSync(sessionId, key, param);
}

export function sessionGetConnSessionId(sessionId: string): string {
  return native.sessionGetConnSessionId(sessionId);
}

export function sessionGetConnToken(sessionId: string): string | null {
  return native.sessionGetConnToken(sessionId);
}

export function sessionGetCustomImageQuality(sessionId: string): Array<number> | null {
  return native.sessionGetCustomImageQuality(sessionId);
}

export function sessionGetDisplaysAsIndividualWindows(sessionId: string): string | null {
  return native.sessionGetDisplaysAsIndividualWindows(sessionId);
}

export function sessionGetEdgeScrollEdgeThickness(sessionId: string): number | null {
  return native.sessionGetEdgeScrollEdgeThickness(sessionId);
}

export function sessionGetEnableTrustedDevices(sessionId: string): boolean {
  return native.sessionGetEnableTrustedDevices(sessionId);
}

export function sessionGetFlutterOption(sessionId: string, key: string): string | null {
  return native.sessionGetFlutterOption(sessionId, key);
}

export function sessionGetImageQuality(sessionId: string): string | null {
  return native.sessionGetImageQuality(sessionId);
}

export function sessionGetIsRecording(sessionId: string): boolean {
  return native.sessionGetIsRecording(sessionId);
}

export function sessionGetKeyboardMode(sessionId: string): string | null {
  return native.sessionGetKeyboardMode(sessionId);
}

export function sessionGetLastAuditNote(sessionId: string): string {
  return native.sessionGetLastAuditNote(sessionId);
}

export function sessionGetOption(sessionId: string, arg: string): string | null {
  return native.sessionGetOption(sessionId, arg);
}

export function sessionGetPeerOption(sessionId: string, name: string): string {
  return native.sessionGetPeerOption(sessionId, name);
}

export function sessionGetRemember(sessionId: string): boolean | null {
  return native.sessionGetRemember(sessionId);
}

export function sessionGetRenderStats(sessionId: string, display: number): string {
  return native.sessionGetRenderStats(sessionId, display);
}

export function sessionGetRemoteAudioState(sessionId: string): string {
  return native.sessionGetRemoteAudioState(sessionId);
}

export function sessionGetReverseMouseWheelSync(sessionId: string): string | null {
  return native.sessionGetReverseMouseWheelSync(sessionId);
}

export function sessionGetRgbaSize(sessionId: string, display: number): number {
  return native.sessionGetRgbaSize(sessionId, display);
}

export function sessionGetScrollStyle(sessionId: string): string | null {
  return native.sessionGetScrollStyle(sessionId);
}

export function sessionGetToggleOption(sessionId: string, arg: string): boolean {
  return native.sessionGetToggleOption(sessionId, arg);
}

export function sessionGetTrackpadSpeed(sessionId: string): number | null {
  return native.sessionGetTrackpadSpeed(sessionId);
}

export function sessionGetUseAllMyDisplaysForTheRemoteSession(sessionId: string): string | null {
  return native.sessionGetUseAllMyDisplaysForTheRemoteSession(sessionId);
}

export function sessionGetViewStyle(sessionId: string): string | null {
  return native.sessionGetViewStyle(sessionId);
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

export function sessionIsKeyboardModeSupported(sessionId: string, mode: string): boolean {
  return native.sessionIsKeyboardModeSupported(sessionId, mode);
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

export function sessionSetAuditGuid(sessionId: string, guid: string): void {
  native.sessionSetAuditGuid(sessionId, guid);
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

export function sessionSetDisplaysAsIndividualWindows(sessionId: string, value: string): void {
  native.sessionSetDisplaysAsIndividualWindows(sessionId, value);
}

export function sessionSetEdgeScrollEdgeThickness(sessionId: string, value: number): void {
  native.sessionSetEdgeScrollEdgeThickness(sessionId, value);
}

export function sessionSetFlutterOption(sessionId: string, key: string, value: string): void {
  native.sessionSetFlutterOption(sessionId, key, value);
}

export function sessionSetImageQuality(sessionId: string, value: string): string {
  return native.sessionSetImageQuality(sessionId, value);
}

export function sessionSetKeyboardMode(sessionId: string, value: string): void {
  native.sessionSetKeyboardMode(sessionId, value);
}

export function sessionSetReverseMouseWheel(sessionId: string, value: string): void {
  native.sessionSetReverseMouseWheel(sessionId, value);
}

export function sessionSetScrollStyle(sessionId: string, value: string): void {
  native.sessionSetScrollStyle(sessionId, value);
}

export function sessionSetShowRemoteCursor(sessionId: string, enabled: boolean): string {
  return native.sessionSetShowRemoteCursor(sessionId, enabled);
}

export function sessionSetSize(sessionId: string, display: number, width: number, height: number): string {
  return native.sessionSetSize(sessionId, display, width, height);
}

export function sessionSetTrackpadSpeed(sessionId: string, value: number): void {
  native.sessionSetTrackpadSpeed(sessionId, value);
}

export function sessionSetUseAllMyDisplaysForTheRemoteSession(sessionId: string, value: string): void {
  native.sessionSetUseAllMyDisplaysForTheRemoteSession(sessionId, value);
}

export function sessionSetViewStyle(sessionId: string, value: string): void {
  native.sessionSetViewStyle(sessionId, value);
}

export function sessionStart(sessionId: string): string {
  return native.sessionStart(sessionId);
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
