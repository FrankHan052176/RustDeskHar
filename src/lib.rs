use hbb_common::{
    message_proto::{
        message as peer_message, Clipboard, ClipboardFormat, Message as PeerMessage,
        MultiClipboards,
    },
    protobuf::Message as _,
    rendezvous_proto::{rendezvous_message, RendezvousMessage, RequestRelay, TestNatRequest},
    Stream,
};
use librustdesk::{
    flutter, flutter_ffi,
    platform::ohos::{self, DirectRenderTarget},
};
use napi_derive_ohos::napi;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::{c_char, c_void, CStr},
    fs,
    net::{IpAddr, SocketAddr, TcpStream},
    slice,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

#[repr(C)]
struct OH_AVCapability {
    _private: [u8; 0],
}

#[cfg(target_env = "ohos")]
#[repr(C)]
struct OH_AVScreenCapture {
    _private: [u8; 0],
}

#[cfg(target_env = "ohos")]
#[repr(C)]
struct OH_AVBuffer {
    _private: [u8; 0],
}
#[cfg(target_env = "ohos")]
#[repr(C)]
struct OH_NativeBuffer {
    _private: [u8; 0],
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_NativeBuffer_Config {
    width: i32,
    height: i32,
    format: i32,
    usage: i32,
    stride: i32,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
struct OH_PixelmapNative {
    _private: [u8; 0],
}
#[cfg(target_env = "ohos")]
#[repr(C)]
struct OH_Pixelmap_ImageInfo {
    _private: [u8; 0],
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_AudioCaptureInfo {
    sample_rate: i32,
    channels: i32,
    source: i32,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_AudioEncInfo {
    bitrate: i32,
    codec: i32,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_AudioInfo {
    mic: OH_AudioCaptureInfo,
    inner: OH_AudioCaptureInfo,
    enc: OH_AudioEncInfo,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct OH_VideoCaptureInfo {
    display_id: u64,
    mission_ids: *mut i32,
    mission_ids_len: i32,
    width: i32,
    height: i32,
    source: i32,
}
#[cfg(target_env = "ohos")]
impl Default for OH_VideoCaptureInfo {
    fn default() -> Self {
        Self {
            display_id: 0,
            mission_ids: std::ptr::null_mut(),
            mission_ids_len: 0,
            width: 0,
            height: 0,
            source: 0,
        }
    }
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_VideoEncInfo {
    codec: i32,
    bitrate: i32,
    frame_rate: i32,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_VideoInfo {
    capture: OH_VideoCaptureInfo,
    enc: OH_VideoEncInfo,
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy)]
struct OH_RecorderInfo {
    url: *mut c_char,
    url_len: u32,
    format: i32,
}
#[cfg(target_env = "ohos")]
impl Default for OH_RecorderInfo {
    fn default() -> Self {
        Self {
            url: std::ptr::null_mut(),
            url_len: 0,
            format: 0,
        }
    }
}
#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct OH_AVScreenCaptureConfig {
    capture_mode: i32,
    data_type: i32,
    audio: OH_AudioInfo,
    video: OH_VideoInfo,
    recorder: OH_RecorderInfo,
}

#[cfg(target_env = "ohos")]
#[link(name = "native_avscreen_capture")]
unsafe extern "C" {
    fn OH_AVScreenCapture_Create() -> *mut OH_AVScreenCapture;
    fn OH_AVScreenCapture_Init(
        capture: *mut OH_AVScreenCapture,
        config: OH_AVScreenCaptureConfig,
    ) -> i32;
    fn OH_AVScreenCapture_SetStateCallback(
        capture: *mut OH_AVScreenCapture,
        callback: unsafe extern "C" fn(*mut OH_AVScreenCapture, i32, *mut c_void),
        user_data: *mut c_void,
    ) -> i32;
    fn OH_AVScreenCapture_SetDataCallback(
        capture: *mut OH_AVScreenCapture,
        callback: unsafe extern "C" fn(
            *mut OH_AVScreenCapture,
            *mut OH_AVBuffer,
            i32,
            i64,
            *mut c_void,
        ),
        user_data: *mut c_void,
    ) -> i32;
    fn OH_AVScreenCapture_SetErrorCallback(
        capture: *mut OH_AVScreenCapture,
        callback: unsafe extern "C" fn(*mut OH_AVScreenCapture, i32, *mut c_void),
        user_data: *mut c_void,
    ) -> i32;
    fn OH_AVScreenCapture_SetMicrophoneEnabled(
        capture: *mut OH_AVScreenCapture,
        enabled: bool,
    ) -> i32;
    fn OH_AVScreenCapture_StartScreenCapture(capture: *mut OH_AVScreenCapture) -> i32;
    fn OH_AVScreenCapture_StopScreenCapture(capture: *mut OH_AVScreenCapture) -> i32;
    fn OH_AVScreenCapture_Release(capture: *mut OH_AVScreenCapture) -> i32;
}

#[cfg(target_env = "ohos")]
#[link(name = "native_media_core")]
unsafe extern "C" {
    fn OH_AVBuffer_GetAddr(buffer: *mut OH_AVBuffer) -> *mut u8;
    fn OH_AVBuffer_GetCapacity(buffer: *mut OH_AVBuffer) -> i32;
    fn OH_AVBuffer_GetNativeBuffer(buffer: *mut OH_AVBuffer) -> *mut OH_NativeBuffer;
}

#[cfg(target_env = "ohos")]
#[link(name = "native_buffer")]
unsafe extern "C" {
    fn OH_NativeBuffer_GetConfig(buffer: *mut OH_NativeBuffer, config: *mut OH_NativeBuffer_Config);
    fn OH_NativeBuffer_Unreference(buffer: *mut OH_NativeBuffer) -> i32;
}

#[cfg(target_env = "ohos")]
#[link(name = "native_display_manager")]
unsafe extern "C" {
    fn OH_NativeDisplayManager_CaptureScreenPixelmap(
        display_id: u32,
        pixelmap: *mut *mut OH_PixelmapNative,
    ) -> i32;
}

#[cfg(target_env = "ohos")]
#[link(name = "pixelmap")]
unsafe extern "C" {
    fn OH_PixelmapImageInfo_Create(info: *mut *mut OH_Pixelmap_ImageInfo) -> i32;
    fn OH_PixelmapImageInfo_GetWidth(info: *mut OH_Pixelmap_ImageInfo, width: *mut u32) -> i32;
    fn OH_PixelmapImageInfo_GetHeight(info: *mut OH_Pixelmap_ImageInfo, height: *mut u32) -> i32;
    fn OH_PixelmapImageInfo_GetRowStride(
        info: *mut OH_Pixelmap_ImageInfo,
        row_stride: *mut u32,
    ) -> i32;
    fn OH_PixelmapImageInfo_GetPixelFormat(
        info: *mut OH_Pixelmap_ImageInfo,
        pixel_format: *mut i32,
    ) -> i32;
    fn OH_PixelmapImageInfo_Release(info: *mut OH_Pixelmap_ImageInfo) -> i32;
    fn OH_PixelmapNative_GetImageInfo(
        pixelmap: *mut OH_PixelmapNative,
        info: *mut OH_Pixelmap_ImageInfo,
    ) -> i32;
    fn OH_PixelmapNative_ReadPixels(
        pixelmap: *mut OH_PixelmapNative,
        destination: *mut u8,
        buffer_size: *mut usize,
    ) -> i32;
    fn OH_PixelmapNative_Release(pixelmap: *mut OH_PixelmapNative) -> i32;
}

#[cfg(target_env = "ohos")]
#[link(name = "ohinput")]
unsafe extern "C" {
    fn OH_Input_RequestInjection(callback: unsafe extern "C" fn(i32)) -> i32;
    fn OH_Input_QueryAuthorizedStatus(status: *mut i32) -> i32;
    fn OH_Input_CancelInjection();
}

#[repr(C)]
enum OH_AVCodecCategory {
    Hardware = 0,
    Software = 1,
}

#[link(name = "native_media_codecbase")]
unsafe extern "C" {
    fn OH_AVCodec_GetCapability(mime: *const c_char, is_encoder: bool) -> *mut OH_AVCapability;
    fn OH_AVCodec_GetCapabilityByCategory(
        mime: *const c_char,
        is_encoder: bool,
        category: OH_AVCodecCategory,
    ) -> *mut OH_AVCapability;
    fn OH_AVCapability_IsHardware(capability: *mut OH_AVCapability) -> bool;
    fn OH_AVCapability_GetName(capability: *mut OH_AVCapability) -> *const c_char;
}

const MIME_VIDEO_AVC: &[u8] = b"video/avc\0";
const MIME_VIDEO_HEVC: &[u8] = b"video/hevc\0";
const MIME_VIDEO_AV1: &[u8] = b"video/av01\0";
const MIME_VIDEO_VP9: &[u8] = b"video/x-vnd.on2.vp9\0";
const MIME_VIDEO_VP8: &[u8] = b"video/x-vnd.on2.vp8\0";

#[cfg(target_env = "ohos")]
#[repr(C)]
struct InputKeyEvent {
    _private: [u8; 0],
}

#[cfg(target_env = "ohos")]
#[repr(C)]
struct InputMouseEvent {
    _private: [u8; 0],
}

#[cfg(target_env = "ohos")]
#[repr(C)]
struct InputTouchEvent {
    _private: [u8; 0],
}

#[cfg(target_env = "ohos")]
#[repr(C)]
#[derive(Default)]
struct OhosTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[cfg(target_env = "ohos")]
#[link(name = "ohinput")]
unsafe extern "C" {
    fn OH_Input_AddKeyEventInterceptor(
        callback: unsafe extern "C" fn(*const InputKeyEvent),
        option: *mut c_void,
    ) -> i32;
    fn OH_Input_RemoveKeyEventInterceptor() -> i32;
    fn OH_Input_GetKeyEventAction(key_event: *const InputKeyEvent) -> i32;
    fn OH_Input_GetKeyEventKeyCode(key_event: *const InputKeyEvent) -> i32;
    fn OH_Input_GetKeyEventActionTime(key_event: *const InputKeyEvent) -> i64;
    fn OH_Input_GetKeyEventWindowId(key_event: *const InputKeyEvent) -> i32;
    fn OH_Input_GetKeyEventDisplayId(key_event: *const InputKeyEvent) -> i32;
    fn OH_Input_CreateMouseEvent() -> *mut InputMouseEvent;
    fn OH_Input_DestroyMouseEvent(mouse_event: *mut *mut InputMouseEvent);
    fn OH_Input_SetMouseEventAction(mouse_event: *mut InputMouseEvent, action: i32);
    fn OH_Input_SetMouseEventDisplayX(mouse_event: *mut InputMouseEvent, display_x: i32);
    fn OH_Input_SetMouseEventDisplayY(mouse_event: *mut InputMouseEvent, display_y: i32);
    fn OH_Input_SetMouseEventButton(mouse_event: *mut InputMouseEvent, button: i32);
    fn OH_Input_SetMouseEventActionTime(mouse_event: *mut InputMouseEvent, action_time: i64);
    fn OH_Input_SetMouseEventDisplayId(mouse_event: *mut InputMouseEvent, display_id: i32);
    fn OH_Input_SetMouseEventGlobalX(mouse_event: *mut InputMouseEvent, global_x: i32);
    fn OH_Input_SetMouseEventGlobalY(mouse_event: *mut InputMouseEvent, global_y: i32);
    fn OH_Input_InjectMouseEventGlobal(mouse_event: *const InputMouseEvent) -> i32;
    fn OH_Input_CreateTouchEvent() -> *mut InputTouchEvent;
    fn OH_Input_DestroyTouchEvent(touch_event: *mut *mut InputTouchEvent) -> i32;
    fn OH_Input_SetTouchEventAction(touch_event: *mut InputTouchEvent, action: i32);
    fn OH_Input_SetTouchEventFingerId(touch_event: *mut InputTouchEvent, finger_id: i32);
    fn OH_Input_SetTouchEventDisplayX(touch_event: *mut InputTouchEvent, display_x: i32);
    fn OH_Input_SetTouchEventDisplayY(touch_event: *mut InputTouchEvent, display_y: i32);
    fn OH_Input_SetTouchEventActionTime(touch_event: *mut InputTouchEvent, action_time: i64);
    fn OH_Input_SetTouchEventDisplayId(touch_event: *mut InputTouchEvent, display_id: i32);
    fn OH_Input_SetTouchEventGlobalX(touch_event: *mut InputTouchEvent, global_x: i32);
    fn OH_Input_SetTouchEventGlobalY(touch_event: *mut InputTouchEvent, global_y: i32);
    fn OH_Input_InjectTouchEventGlobal(touch_event: *const InputTouchEvent) -> i32;
    fn clock_gettime(clock_id: i32, time: *mut OhosTimespec) -> i32;
}

// -------- hilog bridge for the Rust core --------
// On OHOS the upstream `initialize()` never calls `init_log`, so every
// `log::error!/warn!/info!` from the RustDesk core (including the OHOS video
// decoder failures that freeze frames) is silently dropped. Wire the `log`
// facade to OHOS hilog so those messages become visible via
// `hilog -D 0xFF01` (tag "RustDeskCore"). This is diagnostics only.
#[link(name = "hilog_ndk.z")]
unsafe extern "C" {
    fn OH_LOG_PrintMsg(
        log_type: i32,
        level: i32,
        domain: u32,
        tag: *const c_char,
        message: *const c_char,
    ) -> i32;
}

const HILOG_DOMAIN: u32 = 0xFF01;
const HILOG_TAG: &[u8] = b"RustDeskCore\0";
const HILOG_TYPE_APP: i32 = 0;

struct HiLogLogger;

impl log::Log for HiLogLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        // hilog levels: DEBUG=3 INFO=4 WARN=5 ERROR=6 FATAL=7
        let level = match record.level() {
            log::Level::Error => 6,
            log::Level::Warn => 5,
            log::Level::Info => 4,
            log::Level::Debug => 3,
            log::Level::Trace => 3,
        };
        let msg = format!("[{}] {}", record.target(), record.args());
        // Truncate very long lines on a char boundary; hilog has per-message limits.
        let msg: &str = if msg.len() > 3800 {
            let mut end = 3800;
            while end > 0 && !msg.is_char_boundary(end) {
                end -= 1;
            }
            &msg[..end]
        } else {
            msg.as_str()
        };
        if let Ok(c_msg) = std::ffi::CString::new(msg) {
            unsafe {
                let _ = OH_LOG_PrintMsg(
                    HILOG_TYPE_APP,
                    level,
                    HILOG_DOMAIN,
                    HILOG_TAG.as_ptr().cast(),
                    c_msg.as_ptr(),
                );
            }
        }
    }

    fn flush(&self) {}
}

static HILOG_LOGGER: HiLogLogger = HiLogLogger;

fn init_hilog_logger_once() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // Ignore error: another logger already installed (should not happen on OHOS).
        if log::set_logger(&HILOG_LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Info);
            log::info!("hilog logger installed for RustDesk core (domain=0xFF01 tag=RustDeskCore)");
        }
    });
}

const RUSTDESK_UPSTREAM_REPO: &str = "https://github.com/rustdesk/rustdesk";
const NATIVE_PACKAGE_NAME: &str = "rustdesk-ohrs";
const CORE_BLOCKER_MESSAGE: &str = "RustDesk core session calls are available, but Harmony-specific event and frame delivery are still incomplete.";
const REAL_SESSION_BINDING_IMPLEMENTED: bool = true;
const SHOW_REMOTE_CURSOR_OPTION: &str = "show-remote-cursor";
const VIEW_ONLY_OPTION: &str = "view-only";
const CODEC_PREFERENCE_OPTION: &str = "codec-preference";
const ADDRESS_BOOK_SUPERSEDED_MESSAGE: &str = "Server or account changed while synchronizing";
const BUILD_RUSTDESK_SNAPSHOT_PRESENT: &str = env!("BUILD_RUSTDESK_SNAPSHOT_PRESENT");
const BUILD_HBB_COMMON_PRESENT: &str = env!("BUILD_HBB_COMMON_PRESENT");
const BUILD_RUSTDESK_PATH: &str = env!("BUILD_RUSTDESK_PATH");
const BUILD_HBB_COMMON_PATH: &str = env!("BUILD_HBB_COMMON_PATH");
const BUILD_MARKER: &str = env!("BUILD_MARKER");

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static SESSIONS: OnceLock<Mutex<HashMap<String, BridgeSession>>> = OnceLock::new();
static SURFACE_BINDINGS: OnceLock<Mutex<HashMap<(String, u32), SurfaceBinding>>> = OnceLock::new();
static CORE_EVENT_QUEUES: OnceLock<Mutex<HashMap<String, VecDeque<Value>>>> = OnceLock::new();
static RENDER_STATS: OnceLock<Mutex<HashMap<String, HashMap<usize, RenderStats>>>> =
    OnceLock::new();
static INPUT_INTERCEPTOR_ACTIVE: AtomicBool = AtomicBool::new(false);
static INPUT_EVENT_QUEUE: OnceLock<Mutex<VecDeque<Value>>> = OnceLock::new();
static ACCOUNT_JOB: OnceLock<Mutex<BackgroundJsonJob>> = OnceLock::new();
static ACCOUNT_OPTIONS_JOB: OnceLock<Mutex<BackgroundJsonJob>> = OnceLock::new();
static ADDRESS_BOOK_JOB: OnceLock<Mutex<BackgroundJsonJob>> = OnceLock::new();
static ACCOUNT_CHALLENGE: OnceLock<Mutex<Option<AccountChallenge>>> = OnceLock::new();
static ACCOUNT_CHALLENGE_COUNTER: AtomicU64 = AtomicU64::new(1);
static CONTROLLED_RUNTIME: OnceLock<Mutex<ControlledRuntime>> = OnceLock::new();
static CONTROLLED_HOST_LIFECYCLE: OnceLock<Mutex<()>> = OnceLock::new();
static CONTROLLED_CAPTURE_HANDLE: AtomicU64 = AtomicU64::new(0);
static CONTROLLED_AUDIO_CAPTURE_HANDLE: AtomicU64 = AtomicU64::new(0);
static CONTROLLED_CAPTURE_START_PREPARING: AtomicBool = AtomicBool::new(false);
static CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static CONTROLLED_CAPTURE_FALLBACK_RUNNING: AtomicBool = AtomicBool::new(false);
static CONTROLLED_CAPTURE_FALLBACK_THREAD: OnceLock<Mutex<Option<std::thread::JoinHandle<()>>>> =
    OnceLock::new();
static CONTROLLED_INPUT_AUTH_STATUS: AtomicI32 = AtomicI32::new(-1);
static CONTROLLED_INPUT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static CONTROLLED_GLOBAL_MOUSE_X: AtomicI32 = AtomicI32::new(0);
static CONTROLLED_GLOBAL_MOUSE_Y: AtomicI32 = AtomicI32::new(0);
static CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON: AtomicI32 = AtomicI32::new(-1);
static CONTROLLED_GLOBAL_MOUSE_ACTION_TIME: AtomicI64 = AtomicI64::new(0);

const INPUT_MOUSE_ACTION_MOVE: i32 = 1;
const INPUT_MOUSE_ACTION_BUTTON_DOWN: i32 = 2;
const INPUT_MOUSE_ACTION_BUTTON_UP: i32 = 3;
const INPUT_MOUSE_BUTTON_NONE: i32 = -1;
const INPUT_TOUCH_ACTION_DOWN: i32 = 1;
const INPUT_TOUCH_ACTION_MOVE: i32 = 2;
const INPUT_TOUCH_ACTION_UP: i32 = 3;

const MAX_EVENTS_PER_SESSION: usize = 512;
const MAX_INPUT_EVENTS: usize = 256;
const MAX_CONTROLLED_QUEUE_ITEMS: usize = 256;
const MAX_CONTROLLED_JSON_BYTES: usize = 64 * 1024;
const MAX_CONTROLLED_FRAME_BYTES: usize = 32 * 1024 * 1024;
const MAX_CONTROLLED_CLIPBOARD_BYTES: usize = 4 * 1024 * 1024;
const MAX_CONTROLLED_PASSWORD_BYTES: usize = 256;
const CONTROLLED_CAPTURE_LOGICAL_HANDLE: u64 = 1;
const CONTROLLED_PIXELMAP_MAX_DIMENSION: usize = 1_920;
const CONTROLLED_CAPTURE_FALLBACK_FRAME_INTERVAL: Duration = Duration::from_millis(200);
const CONTROLLED_CAPTURE_FALLBACK_START_DELAY: Duration = Duration::from_millis(1200);
const DISPLAY_MANAGER_OK: i32 = 0;
const IMAGE_SUCCESS: i32 = 0;
const PIXEL_FORMAT_RGBA_8888: i32 = 3;
const PIXEL_FORMAT_BGRA_8888: i32 = 4;
const INPUT_SUCCESS: i32 = 0;
const INPUT_REPEAT_INTERCEPTOR: i32 = 4_200_001;

#[derive(Default)]
struct BackgroundJsonJob {
    running: bool,
    result: Option<Value>,
    generation: u64,
}

#[derive(Clone)]
struct AccountChallenge {
    id: String,
    api_server: String,
    username: String,
    secret: String,
    challenge_type: String,
}

struct ApiResponse {
    status_code: u16,
    body: Value,
}

#[derive(Default)]
struct ControlledRuntime {
    running: bool,
    generation: u64,
    server_config: Value,
    capabilities: Value,
    screen_config: Value,
    audio_config: Value,
    audio_enabled: bool,
    incoming: VecDeque<Value>,
    input: VecDeque<Value>,
    clipboard: VecDeque<Value>,
    pushed_screen_frames: u64,
    pushed_audio_frames: u64,
    native_capture_state: i32,
    native_capture_started: bool,
    native_capture_error: i32,
    native_capture_frames: u64,
    native_capture_bytes: u64,
    native_capture_audio_frames: u64,
    native_capture_audio_bytes: u64,
    native_capture_last_timestamp: i64,
    screenshot_fallback_active: bool,
    screenshot_fallback_frames: u64,
    screenshot_fallback_errors: u64,
    last_error: Option<String>,
}

fn controlled_runtime() -> &'static Mutex<ControlledRuntime> {
    CONTROLLED_RUNTIME.get_or_init(|| Mutex::new(ControlledRuntime::default()))
}

fn controlled_host_lifecycle() -> &'static Mutex<()> {
    CONTROLLED_HOST_LIFECYCLE.get_or_init(|| Mutex::new(()))
}

#[cfg(target_env = "ohos")]
fn controlled_native_capture_is_healthy() -> bool {
    if CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS.load(Ordering::Acquire)
        || CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) == 0
        || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) == 0
    {
        return false;
    }
    controlled_runtime()
        .lock()
        .map(|state| {
            state.native_capture_error == 0
                && state.native_capture_started
                && state.native_capture_frames > 0
                && state.last_error.is_none()
        })
        .unwrap_or(false)
}

#[cfg(not(target_env = "ohos"))]
fn controlled_native_capture_is_healthy() -> bool {
    false
}

fn controlled_parse_json(action: &str, input: &str, max_bytes: usize) -> Result<Value, String> {
    if input.len() > max_bytes {
        return Err(format!("{} payload exceeds {} bytes", action, max_bytes));
    }
    serde_json::from_str(input)
        .map_err(|err| format!("{} payload is invalid JSON: {}", action, err))
}

fn controlled_clients_payload(generation: u64) -> (Vec<Value>, Vec<Value>) {
    let raw =
        serde_json::from_str::<Value>(&ohos::host_clients_state()).unwrap_or_else(|_| json!([]));
    let clients = raw
        .as_array()
        .into_iter()
        .flatten()
        .map(|client| {
            let id = client.get("id").and_then(Value::as_i64).unwrap_or_default();
            json!({
              "requestId":format!("{}:{}", generation, id),
              "peerId":client.get("peer_id").and_then(Value::as_str).unwrap_or_default(),
              "peerName":client.get("name").and_then(Value::as_str).unwrap_or_default(),
              "authorized":client.get("authorized").and_then(Value::as_bool).unwrap_or(false),
              "disconnected":client.get("disconnected").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();
    let requests = clients
        .iter()
        .filter(|client| {
            !client
                .get("authorized")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !client
                    .get("disconnected")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .cloned()
        .collect();
    (clients, requests)
}

fn controlled_harmony_key_code_from_char(value: u64) -> Option<u64> {
    match value {
        value @ 48..=57 => Some(2000 + (value - 48)),
        value @ 65..=90 => Some(2017 + (value - 65)),
        value @ 97..=122 => Some(2017 + (value - 97)),
        44 => Some(2043),
        46 => Some(2044),
        32 => Some(2050),
        10 | 13 => Some(2054),
        8 => Some(2055),
        96 => Some(2056),
        45 => Some(2057),
        61 => Some(2058),
        91 => Some(2059),
        93 => Some(2060),
        92 => Some(2061),
        59 => Some(2062),
        39 => Some(2063),
        47 => Some(2064),
        _ => None,
    }
}

fn controlled_harmony_key_code_from_name(name: &str) -> Option<u64> {
    if let Some(number) = name
        .strip_prefix('F')
        .and_then(|value| value.parse::<u64>().ok())
    {
        if (1..=12).contains(&number) {
            return Some(2089 + number);
        }
    }
    if let Some(number) = name
        .strip_prefix("Numpad")
        .and_then(|value| value.parse::<u64>().ok())
    {
        if number <= 9 {
            return Some(2103 + number);
        }
    }
    Some(match name {
        "UpArrow" => 2012,
        "DownArrow" => 2013,
        "LeftArrow" => 2014,
        "RightArrow" => 2015,
        "Alt" | "Option" | "Menu" => 2045,
        "RAlt" => 2046,
        "Shift" => 2047,
        "RShift" => 2048,
        "Tab" => 2049,
        "Space" => 2050,
        "Return" => 2054,
        "Backspace" => 2055,
        "Apps" => 2067,
        "PageUp" => 2068,
        "PageDown" => 2069,
        "Escape" | "Cancel" => 2070,
        "Delete" => 2071,
        "Control" => 2072,
        "RControl" => 2073,
        "CapsLock" => 2074,
        "Scroll" => 2075,
        "Meta" => 2076,
        "RWin" => 2077,
        "Snapshot" | "Print" => 2079,
        "Pause" => 2080,
        "Home" => 2081,
        "End" => 2082,
        "Insert" => 2083,
        "NumLock" => 2102,
        "Divide" => 2113,
        "Multiply" => 2114,
        "Subtract" => 2115,
        "Add" => 2116,
        "Decimal" => 2117,
        "NumpadEnter" => 2119,
        "Equals" => 2058,
        _ => return None,
    })
}

fn controlled_harmony_key_code_from_usb_hid(value: u64) -> Option<u64> {
    match value {
        0x04..=0x1D => Some(2017 + value - 0x04),
        0x1E..=0x26 => Some(2001 + value - 0x1E),
        0x27 => Some(2000),
        0x28 => Some(2054),
        0x29 => Some(2070),
        0x2A => Some(2055),
        0x2B => Some(2049),
        0x2C => Some(2050),
        0x2D => Some(2057),
        0x2E => Some(2058),
        0x2F => Some(2059),
        0x30 => Some(2060),
        0x31 => Some(2061),
        0x33 => Some(2062),
        0x34 => Some(2063),
        0x35 => Some(2056),
        0x36 => Some(2043),
        0x37 => Some(2044),
        0x38 => Some(2064),
        0x39 => Some(2074),
        0x3A..=0x45 => Some(2090 + value - 0x3A),
        0x46 => Some(2079),
        0x47 => Some(2075),
        0x48 => Some(2080),
        0x49 => Some(2083),
        0x4A => Some(2081),
        0x4B => Some(2068),
        0x4C => Some(2071),
        0x4D => Some(2082),
        0x4E => Some(2069),
        0x4F => Some(2015),
        0x50 => Some(2014),
        0x51 => Some(2013),
        0x52 => Some(2012),
        0x53 => Some(2102),
        0x54 => Some(2113),
        0x55 => Some(2114),
        0x56 => Some(2115),
        0x57 => Some(2116),
        0x58 => Some(2119),
        0x59..=0x61 => Some(2104 + value - 0x59),
        0x62 => Some(2103),
        0x63 => Some(2117),
        0xE0 => Some(2072),
        0xE1 => Some(2047),
        0xE2 => Some(2045),
        0xE3 => Some(2076),
        0xE4 => Some(2073),
        0xE5 => Some(2048),
        0xE6 => Some(2046),
        0xE7 => Some(2077),
        _ => None,
    }
}

fn controlled_harmony_key_code_from_linux_evdev(value: u64) -> Option<u64> {
    Some(match value {
        1 => 2070,
        2..=10 => 2001 + value - 2,
        11 => 2000,
        14 => 2055,
        15 => 2049,
        16 => 2033,
        17 => 2039,
        18 => 2021,
        19 => 2034,
        20 => 2036,
        21 => 2041,
        22 => 2037,
        23 => 2025,
        24 => 2031,
        25 => 2032,
        26 => 2059,
        27 => 2060,
        28 => 2054,
        29 => 2072,
        30 => 2017,
        31 => 2035,
        32 => 2020,
        33 => 2022,
        34 => 2023,
        35 => 2024,
        36 => 2026,
        37 => 2027,
        38 => 2028,
        39 => 2062,
        40 => 2063,
        41 => 2056,
        42 => 2047,
        43 => 2061,
        44 => 2042,
        45 => 2040,
        46 => 2019,
        47 => 2038,
        48 => 2018,
        49 => 2030,
        50 => 2029,
        51 => 2043,
        52 => 2044,
        53 => 2064,
        54 => 2048,
        56 => 2045,
        57 => 2050,
        58 => 2074,
        59..=68 => 2090 + value - 59,
        87 => 2100,
        88 => 2101,
        97 => 2073,
        100 => 2046,
        102 => 2081,
        103 => 2012,
        104 => 2068,
        105 => 2014,
        106 => 2015,
        107 => 2082,
        108 => 2013,
        109 => 2069,
        110 => 2083,
        111 => 2071,
        125 => 2076,
        126 => 2077,
        _ => return None,
    })
}

fn controlled_harmony_key_code_from_linux_x11(value: u64) -> Option<u64> {
    controlled_harmony_key_code_from_linux_evdev(value.checked_sub(8)?)
}

fn controlled_mouse_button(rustdesk_button: i32) -> Option<i32> {
    match rustdesk_button {
        1 => Some(0),
        2 => Some(2),
        4 => Some(1),
        8 => Some(6),
        16 => Some(5),
        _ => None,
    }
}

fn controlled_pointer_coordinate(value: i64, stream_size: u64, source_size: u64) -> i64 {
    if stream_size == 0 || source_size == 0 || stream_size == source_size {
        return value;
    }
    value
        .max(0)
        .saturating_mul(source_size as i64)
        .checked_div(stream_size as i64)
        .unwrap_or(value)
        .min(source_size.saturating_sub(1) as i64)
}

fn controlled_input_events_from_core(
    event: &Value,
    sequence: String,
    display_id: u64,
    stream_size: (u64, u64),
    source_size: (u64, u64),
) -> Vec<Value> {
    match event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "pointer" => {
            let kind = event.get("kind").and_then(Value::as_str).unwrap_or("mouse");
            let mask = event
                .get("mask")
                .and_then(Value::as_i64)
                .unwrap_or_default() as i32;
            let event_type = mask & 0x7;
            let x = event.get("x").and_then(Value::as_i64).unwrap_or_default();
            let y = event.get("y").and_then(Value::as_i64).unwrap_or_default();
            let display_x = controlled_pointer_coordinate(x, stream_size.0, source_size.0);
            let display_y = controlled_pointer_coordinate(y, stream_size.1, source_size.1);
            let input_type = if kind == "touch" { "touch" } else { "mouse" };
            if input_type == "touch" {
                let action = match mask {
                    4 => "down",
                    5 => "move",
                    6 => "up",
                    _ => return Vec::new(),
                };
                return vec![
                    json!({"sequence":sequence.clone(),"eventId":sequence,"type":"touch","action":action,
                  "displayId":display_id,"x":display_x,"y":display_y,"touchId":0}),
                ];
            }
            match event_type {
                0 => vec![
                    json!({"sequence":sequence.clone(),"eventId":sequence,"type":"mouse","action":"move",
                  "displayId":display_id,"x":display_x,"y":display_y}),
                ],
                1 | 2 => {
                    let Some(button) = controlled_mouse_button(mask >> 3) else {
                        return Vec::new();
                    };
                    vec![
                        json!({"sequence":sequence.clone(),"eventId":sequence,"type":"mouse",
                      "action":if event_type == 1 { "button_down" } else { "button_up" },
                      "displayId":display_id,"x":display_x,"y":display_y,"button":button}),
                    ]
                }
                3 | 4 => {
                    let action = if event_type == 3 { "wheel" } else { "trackpad" };
                    let mut axis_events = Vec::with_capacity(2);
                    if y != 0 {
                        let event_id = format!("{sequence}:v");
                        axis_events.push(json!({"sequence":event_id.clone(),"eventId":event_id,"type":"mouse",
                          "action":action,"displayId":display_id,"axis":0,"value":y.saturating_neg()}));
                    }
                    if x != 0 {
                        let event_id = format!("{sequence}:h");
                        axis_events.push(json!({"sequence":event_id.clone(),"eventId":event_id,"type":"mouse",
                          "action":action,"displayId":display_id,"axis":1,"value":x.saturating_neg()}));
                    }
                    axis_events
                }
                // InputKit exposes absolute mouse movement only. Do not pretend
                // relative type 5 events were injected successfully.
                _ => Vec::new(),
            }
        }
        "key" => {
            let press = event.get("press").and_then(Value::as_bool).unwrap_or(false);
            let down = event.get("down").and_then(Value::as_bool).unwrap_or(false);
            let action = if press {
                "press"
            } else if down {
                "down"
            } else {
                "up"
            };
            let mode = event
                .get("mode")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let union_kind = event
                .get("unionKind")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let key_code = match union_kind {
                "controlKey" => event
                    .get("controlKeyName")
                    .and_then(Value::as_str)
                    .and_then(controlled_harmony_key_code_from_name),
                // RustDesk map/translate mode stores a platform-specific physical
                // key code in `chr`. For HarmonyOS peers the client sends Linux
                // evdev key codes here, not USB HID usages. Prefer the printable
                // `seq` value for normal text keys; only use `chr` as an ASCII
                // fallback. Treating Linux codes 30/31/32 as HID produced 9/0/Enter
                // for A/S/D.
                "chr" => event
                    .get("seq")
                    .and_then(Value::as_str)
                    .and_then(|value| {
                        let mut chars = value.chars();
                        let first = chars.next()?;
                        if chars.next().is_none() {
                            controlled_harmony_key_code_from_char(first as u64)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        event.get("chr").and_then(Value::as_u64).and_then(|value| {
                            // Map, Translate and Auto may all carry the physical
                            // key produced by the sender-side map path while
                            // retaining their configured mode value. For a
                            // HarmonyOS peer that physical key is a Linux/X11
                            // keycode (evdev + 8). Only Legacy mode uses `chr`
                            // as a character value.
                            if mode != 0 {
                                controlled_harmony_key_code_from_linux_x11(value)
                            } else {
                                controlled_harmony_key_code_from_char(value)
                            }
                        })
                    }),
                "usbHid" => event
                    .get("usbHid")
                    .or_else(|| event.get("chr"))
                    .and_then(Value::as_u64)
                    .and_then(controlled_harmony_key_code_from_usb_hid),
                "unicode" => event
                    .get("unicode")
                    .and_then(Value::as_u64)
                    .and_then(controlled_harmony_key_code_from_char),
                "seq" => event.get("seq").and_then(Value::as_str).and_then(|value| {
                    let mut chars = value.chars();
                    let first = chars.next()?;
                    if chars.next().is_none() {
                        controlled_harmony_key_code_from_char(first as u64)
                    } else {
                        None
                    }
                }),
                _ => None,
            };
            let text = event
                .get("seq")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    event
                        .get("unicode")
                        .and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                        .and_then(char::from_u32)
                        .map(|value| value.to_string())
                });
            vec![
                json!({"sequence":sequence.clone(),"eventId":sequence,"type":"key","action":action,
              "keyCode":key_code,"value":event.get("unicode"),"keyKind":event.get("unionKind"),
              "keyName":event.get("controlKeyName"),"text":text,"mode":event.get("mode"),
              "modeName":event.get("modeName"),"modifiers":event.get("modifiers"),
              "modifierNames":event.get("modifierNames")}),
            ]
        }
        _ => Vec::new(),
    }
}

fn controlled_view_only_config_is_valid(config: &Value) -> bool {
    !config
        .get("enableInput")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !config
            .get("enableClipboard")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && config
            .get("enableAudio")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn controlled_view_only_permission_is_valid(permission: &str, enabled: bool) -> bool {
    if permission == "audio" {
        return enabled;
    }
    if matches!(
        permission,
        "keyboard" | "clipboard" | "file" | "restart" | "recording"
    ) {
        return !enabled;
    }
    true
}

#[cfg(test)]
mod controlled_input_tests {
    use super::*;

    #[test]
    fn view_only_host_rejects_input_and_clipboard_escalation() {
        assert!(controlled_view_only_config_is_valid(&json!({
            "enableInput": false,
            "enableClipboard": false,
            "enableAudio": true
        })));
        assert!(!controlled_view_only_config_is_valid(
            &json!({"enableInput": true})
        ));
        assert!(!controlled_view_only_config_is_valid(
            &json!({"enableClipboard": true})
        ));
        assert!(!controlled_view_only_config_is_valid(&json!({
            "enableInput": false,
            "enableClipboard": false,
            "enableAudio": false
        })));
        assert!(!controlled_view_only_permission_is_valid("keyboard", true));
        assert!(!controlled_view_only_permission_is_valid("clipboard", true));
        assert!(!controlled_view_only_permission_is_valid("file", true));
        assert!(!controlled_view_only_permission_is_valid("restart", true));
        assert!(!controlled_view_only_permission_is_valid("recording", true));
        assert!(!controlled_view_only_permission_is_valid("audio", false));
        assert!(controlled_view_only_permission_is_valid("keyboard", false));
        assert!(controlled_view_only_permission_is_valid("audio", true));
    }

    #[test]
    fn wheel_preserves_both_axes_and_normalizes_direction() {
        let events = controlled_input_events_from_core(
            &json!({"type":"pointer","kind":"mouse","mask":3,"x":10,"y":-20}),
            "7".to_owned(),
            0,
            (1920, 1280),
            (3120, 2080),
        );
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["axis"], json!(0));
        assert_eq!(events[0]["value"], json!(20));
        assert_eq!(events[1]["axis"], json!(1));
        assert_eq!(events[1]["value"], json!(-10));
    }

    #[test]
    fn wheel_touch_fallback_moves_against_scroll_direction() {
        assert_eq!(
            controlled_wheel_touch_points(0, 100, 200, 1.0),
            [(100, 248), (100, 224), (100, 200), (100, 176), (100, 152)]
        );
        assert_eq!(
            controlled_wheel_touch_points(1, 100, 200, -1.0),
            [(52, 200), (76, 200), (100, 200), (124, 200), (148, 200)]
        );
    }

    #[test]
    fn button_mapping_never_falls_back_to_left() {
        let unsupported = controlled_input_events_from_core(
            &json!({"type":"pointer","kind":"mouse","mask":257,"x":1,"y":2}),
            "8".to_owned(),
            0,
            (1920, 1280),
            (3120, 2080),
        );
        assert!(unsupported.is_empty());

        let back = controlled_input_events_from_core(
            &json!({"type":"pointer","kind":"mouse","mask":65,"x":1,"y":2}),
            "9".to_owned(),
            0,
            (1920, 1280),
            (3120, 2080),
        );
        assert_eq!(back[0]["button"], json!(6));
    }

    #[test]
    fn absolute_pointer_coordinates_scale_to_physical_display() {
        let events = controlled_input_events_from_core(
            &json!({"type":"pointer","kind":"mouse","mask":0,"x":960,"y":640}),
            "10".to_owned(),
            0,
            (1920, 1280),
            (3120, 2080),
        );
        assert_eq!(events[0]["x"], json!(1560));
        assert_eq!(events[0]["y"], json!(1040));
    }

    #[test]
    fn inputkit_side_buttons_map_to_native_global_buttons() {
        assert_eq!(controlled_native_mouse_button(0), Some(0));
        assert_eq!(controlled_native_mouse_button(1), Some(1));
        assert_eq!(controlled_native_mouse_button(2), Some(2));
        assert_eq!(controlled_native_mouse_button(5), Some(3));
        assert_eq!(controlled_native_mouse_button(6), Some(4));
        assert_eq!(controlled_native_mouse_button(9), None);
    }

    #[test]
    fn non_legacy_keys_remove_linux_x11_offset_in_all_configured_modes() {
        let cases = [
            (38, "a", 2017),
            (39, "s", 2035),
            (40, "d", 2020),
            (22, "backspace", 2055),
        ];
        for mode in [1, 2, 3] {
            for (chr, seq, expected) in cases {
                let events = controlled_input_events_from_core(
                    &json!({
                        "type":"key",
                        "press":false,
                        "down":true,
                        "mode":mode,
                        "unionKind":"chr",
                        "chr":chr
                    }),
                    format!("key-{mode}-{seq}"),
                    0,
                    (1920, 1280),
                    (3120, 2080),
                );
                assert_eq!(events[0]["keyCode"], json!(expected));
            }
        }
    }
}

fn controlled_response(action: &str, ok: bool, state: &ControlledRuntime, extra: Value) -> String {
    let mut response = json!({
      "ok": ok,
      "action": action,
      "running": state.running,
      "generation": state.generation,
      "coreHostBridgeAvailable": true,
      "nativeScreenCaptureAvailable": cfg!(target_env = "ohos"),
      "lastError": state.last_error,
    });
    if let (Some(target), Some(source)) = (response.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    response.to_string()
}

fn controlled_view_only_denial(action: &str) -> String {
    json!({
        "ok": false,
        "action": action,
        "message": "HarmonyOS watched/view-only hosting does not expose control or mutable media injection"
    })
    .to_string()
}

#[cfg(target_env = "ohos")]
fn controlled_capture_state_is_terminal(state_code: i32) -> bool {
    // 11 and 13 are pause states on newer systems. A watched host must not
    // continue serving stale frames while system capture is paused.
    matches!(state_code, 1 | 2 | 3 | 4 | 10 | 11 | 13)
}

#[cfg(target_env = "ohos")]
unsafe fn schedule_controlled_capture_cleanup(capture: *mut OH_AVScreenCapture) {
    let handle = capture as usize as u64;
    if handle == 0 {
        return;
    }
    if CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    if CONTROLLED_AUDIO_CAPTURE_HANDLE
        .compare_exchange(handle, 0, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS.store(false, Ordering::Release);
        return;
    }
    std::thread::spawn(move || {
        let capture = handle as usize as *mut OH_AVScreenCapture;
        let stop_code = unsafe { OH_AVScreenCapture_StopScreenCapture(capture) };
        let release_code = unsafe { OH_AVScreenCapture_Release(capture) };
        CONTROLLED_CAPTURE_HANDLE.store(0, Ordering::Release);
        stop_controlled_capture_fallback();
        ohos::stop_host();
        if stop_code != 0 || release_code != 0 {
            if release_code != 0 {
                if CONTROLLED_AUDIO_CAPTURE_HANDLE
                    .compare_exchange(0, handle, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    CONTROLLED_CAPTURE_HANDLE
                        .store(CONTROLLED_CAPTURE_LOGICAL_HANDLE, Ordering::Release);
                }
            }
            if let Ok(mut state) = controlled_runtime().lock() {
                state.last_error = Some(format!(
                    "screen and inner-audio capture cleanup failed: stop={} release={}",
                    stop_code, release_code
                ));
                state.running = false;
                state.audio_enabled = false;
                state.native_capture_started = false;
            }
        } else if let Ok(mut state) = controlled_runtime().lock() {
            state.running = false;
            state.audio_enabled = false;
            state.native_capture_started = false;
        }
        CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS.store(false, Ordering::Release);
    });
}

#[cfg(target_env = "ohos")]
unsafe extern "C" fn controlled_capture_state_callback(
    capture: *mut OH_AVScreenCapture,
    state_code: i32,
    _user_data: *mut c_void,
) {
    let handle = capture as usize as u64;
    if handle == 0 || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != handle {
        return;
    }
    if let Ok(mut state) = controlled_runtime().lock() {
        state.native_capture_state = state_code;
        if state_code == 0 {
            state.native_capture_started = true;
        } else if controlled_capture_state_is_terminal(state_code) {
            state.native_capture_started = false;
        }
    }
    if controlled_capture_state_is_terminal(state_code) {
        unsafe { schedule_controlled_capture_cleanup(capture) };
    }
}

#[cfg(target_env = "ohos")]
unsafe extern "C" fn controlled_capture_error_callback(
    capture: *mut OH_AVScreenCapture,
    error_code: i32,
    _user_data: *mut c_void,
) {
    let handle = capture as usize as u64;
    if handle == 0 || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != handle {
        return;
    }
    if let Ok(mut state) = controlled_runtime().lock() {
        state.native_capture_error = error_code;
        state.native_capture_started = false;
        state.last_error = Some(format!("OH_AVScreenCapture error {}", error_code));
    }
    unsafe { schedule_controlled_capture_cleanup(capture) };
}

#[cfg(target_env = "ohos")]
unsafe extern "C" fn controlled_capture_data_callback(
    capture: *mut OH_AVScreenCapture,
    buffer: *mut OH_AVBuffer,
    buffer_type: i32,
    timestamp: i64,
    _user_data: *mut c_void,
) {
    let handle = capture as usize as u64;
    if handle == 0 || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != handle {
        return;
    }
    if buffer.is_null() || (buffer_type != 0 && buffer_type != 1) {
        return;
    }
    let capacity = unsafe { OH_AVBuffer_GetCapacity(buffer) };
    let addr = unsafe { OH_AVBuffer_GetAddr(buffer) };
    if capacity <= 0 || addr.is_null() || capacity as usize > MAX_CONTROLLED_FRAME_BYTES {
        return;
    }
    if buffer_type == 1 {
        let audio_enabled = controlled_runtime()
            .lock()
            .map(|state| state.audio_enabled)
            .unwrap_or(false);
        if !audio_enabled {
            return;
        }
        // Original inner-audio buffers are PCM S16LE. The native buffer is
        // callback-owned, so convert complete stereo frames into an owned f32 LE
        // byte vector and synchronously hand it to Core before returning.
        let input = unsafe { slice::from_raw_parts(addr, capacity as usize) };
        let stereo_bytes = input.len() - (input.len() % 4);
        if stereo_bytes == 0 {
            return;
        }
        let mut output = Vec::with_capacity(stereo_bytes * 2);
        for sample in input[..stereo_bytes].chunks_exact(2) {
            let normalized = i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32768.0;
            output.extend_from_slice(&normalized.to_le_bytes());
        }
        ohos::push_host_audio_f32_stereo(&output);
        if let Ok(mut state) = controlled_runtime().lock() {
            state.pushed_audio_frames = state.pushed_audio_frames.saturating_add(1);
            state.native_capture_audio_frames = state.native_capture_audio_frames.saturating_add(1);
            state.native_capture_audio_bytes = state
                .native_capture_audio_bytes
                .saturating_add(stereo_bytes as u64);
        }
        return;
    }
    let (width, height) = controlled_runtime()
        .lock()
        .ok()
        .map(|state| {
            (
                state
                    .screen_config
                    .get("width")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
                state
                    .screen_config
                    .get("height")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            )
        })
        .unwrap_or_default();
    let expected_bytes = width.saturating_mul(height).saturating_mul(4);
    let native_buffer = unsafe { OH_AVBuffer_GetNativeBuffer(buffer) };
    let mut packed_rgba = Vec::new();
    if !native_buffer.is_null() && expected_bytes > 0 {
        let mut native_config = OH_NativeBuffer_Config::default();
        unsafe { OH_NativeBuffer_GetConfig(native_buffer, &mut native_config) };
        let row_bytes = width.saturating_mul(4);
        let stride = usize::try_from(native_config.stride).unwrap_or(0);
        let native_width = usize::try_from(native_config.width).unwrap_or(0);
        let native_height = usize::try_from(native_config.height).unwrap_or(0);
        let required_bytes = stride
            .saturating_mul(height.saturating_sub(1))
            .saturating_add(row_bytes);
        if row_bytes > 0
            && stride >= row_bytes
            && native_width >= width
            && native_height >= height
            && required_bytes <= capacity as usize
            && expected_bytes <= MAX_CONTROLLED_FRAME_BYTES
        {
            let source = unsafe { slice::from_raw_parts(addr, capacity as usize) };
            packed_rgba.reserve_exact(expected_bytes);
            for row in 0..height {
                let start = row.saturating_mul(stride);
                packed_rgba.extend_from_slice(&source[start..start + row_bytes]);
            }
        }
        unsafe {
            OH_NativeBuffer_Unreference(native_buffer);
        }
    }
    let forwarded = packed_rgba.len() == expected_bytes
        && ohos::push_host_screen_frame_rgba(&packed_rgba, width, height);
    if let Ok(mut state) = controlled_runtime().lock() {
        if forwarded {
            state.native_capture_frames = state.native_capture_frames.saturating_add(1);
            state.native_capture_bytes = state
                .native_capture_bytes
                .saturating_add(expected_bytes as u64);
            state.native_capture_last_timestamp = timestamp;
        } else {
            state.native_capture_error = -1;
            state.native_capture_started = false;
            state.last_error = Some("Invalid or rejected native screen capture frame".to_string());
        }
    }
    if !forwarded {
        unsafe { schedule_controlled_capture_cleanup(capture) };
    }
}

#[cfg(target_env = "ohos")]
unsafe fn start_controlled_av_capture(
    width: i32,
    height: i32,
    display_id: u64,
    frame_rate: i32,
) -> Result<(), String> {
    if CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS.load(Ordering::Acquire) {
        return Err("previous screen capture cleanup is still in progress".to_string());
    }
    if CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0 {
        return Ok(());
    }
    let capture = unsafe { OH_AVScreenCapture_Create() };
    if capture.is_null() {
        return Err("OH_AVScreenCapture_Create returned null".to_string());
    }
    let config = OH_AVScreenCaptureConfig {
        capture_mode: 1,
        data_type: 0,
        audio: OH_AudioInfo {
            mic: OH_AudioCaptureInfo::default(),
            inner: OH_AudioCaptureInfo {
                sample_rate: 48_000,
                channels: 2,
                source: 2,
            },
            enc: OH_AudioEncInfo::default(),
        },
        video: OH_VideoInfo {
            capture: OH_VideoCaptureInfo {
                display_id,
                mission_ids: std::ptr::null_mut(),
                mission_ids_len: 0,
                width,
                height,
                source: 2,
            },
            enc: OH_VideoEncInfo {
                codec: 0,
                bitrate: 0,
                frame_rate,
            },
        },
        recorder: OH_RecorderInfo::default(),
    };
    let microphone_code = unsafe { OH_AVScreenCapture_SetMicrophoneEnabled(capture, false) };
    let state_code = unsafe {
        OH_AVScreenCapture_SetStateCallback(
            capture,
            controlled_capture_state_callback,
            std::ptr::null_mut(),
        )
    };
    let data_code = unsafe {
        OH_AVScreenCapture_SetDataCallback(
            capture,
            controlled_capture_data_callback,
            std::ptr::null_mut(),
        )
    };
    let error_code = unsafe {
        OH_AVScreenCapture_SetErrorCallback(
            capture,
            controlled_capture_error_callback,
            std::ptr::null_mut(),
        )
    };
    let init_code = unsafe { OH_AVScreenCapture_Init(capture, config) };
    if microphone_code != 0
        || state_code != 0
        || data_code != 0
        || error_code != 0
        || init_code != 0
    {
        unsafe {
            OH_AVScreenCapture_Release(capture);
        }
        return Err(format!(
            "screen and inner-audio capture setup failed: microphone={} state={} data={} error={} init={}",
            microphone_code, state_code, data_code, error_code, init_code
        ));
    }
    let handle = capture as usize as u64;
    CONTROLLED_AUDIO_CAPTURE_HANDLE.store(handle, Ordering::Release);
    let start_code = unsafe { OH_AVScreenCapture_StartScreenCapture(capture) };
    if start_code != 0 {
        if CONTROLLED_AUDIO_CAPTURE_HANDLE
            .compare_exchange(handle, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            unsafe {
                OH_AVScreenCapture_Release(capture);
            }
        }
        return Err(format!(
            "screen and inner-audio capture start failed: {}",
            start_code
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != handle {
            return Err(
                "screen and inner-audio capture was interrupted during startup".to_string(),
            );
        }
        let (started, frames, native_error, last_error) = controlled_runtime()
            .lock()
            .map(|state| {
                (
                    state.native_capture_started,
                    state.native_capture_frames,
                    state.native_capture_error,
                    state.last_error.clone(),
                )
            })
            .unwrap_or((false, 0, -1, Some("capture state lock failed".to_string())));
        if native_error != 0 {
            return Err(last_error.unwrap_or_else(|| {
                format!("screen capture failed with native error {}", native_error)
            }));
        }
        if started && frames > 0 {
            return Ok(());
        }
        if Instant::now() >= deadline {
            if CONTROLLED_AUDIO_CAPTURE_HANDLE
                .compare_exchange(handle, 0, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                unsafe {
                    OH_AVScreenCapture_StopScreenCapture(capture);
                    OH_AVScreenCapture_Release(capture);
                }
            }
            return Err("system screen capture confirmation or first frame timed out".to_string());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(target_env = "ohos")]
unsafe fn stop_controlled_av_capture() -> Result<(i32, i32), String> {
    let handle = CONTROLLED_AUDIO_CAPTURE_HANDLE.swap(0, Ordering::AcqRel);
    if handle == 0 {
        return Ok((0, 0));
    }
    let capture = handle as usize as *mut OH_AVScreenCapture;
    let stop_code = unsafe { OH_AVScreenCapture_StopScreenCapture(capture) };
    let release_code = unsafe { OH_AVScreenCapture_Release(capture) };
    if release_code != 0 {
        let _ = CONTROLLED_AUDIO_CAPTURE_HANDLE.compare_exchange(
            0,
            handle,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
    if stop_code != 0 || release_code != 0 {
        return Err(format!(
            "screen and inner-audio capture stop failed: stop={} release={}",
            stop_code, release_code
        ));
    }
    Ok((stop_code, release_code))
}

#[cfg(target_env = "ohos")]
fn controlled_capture_fallback_thread() -> &'static Mutex<Option<std::thread::JoinHandle<()>>> {
    CONTROLLED_CAPTURE_FALLBACK_THREAD.get_or_init(|| Mutex::new(None))
}

#[cfg(target_env = "ohos")]
fn stop_controlled_capture_fallback() {
    CONTROLLED_CAPTURE_FALLBACK_RUNNING.store(false, Ordering::Release);
    let handle = controlled_capture_fallback_thread()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(handle) = handle {
        let _ = handle.join();
    }
    if let Ok(mut state) = controlled_runtime().lock() {
        state.screenshot_fallback_active = false;
    }
}

#[cfg(target_env = "ohos")]
unsafe fn capture_display_pixelmap_rgba(
    display_id: u32,
) -> Result<(Vec<u8>, usize, usize), String> {
    let mut pixelmap = std::ptr::null_mut();
    let capture_code =
        unsafe { OH_NativeDisplayManager_CaptureScreenPixelmap(display_id, &mut pixelmap) };
    if capture_code != DISPLAY_MANAGER_OK || pixelmap.is_null() {
        return Err(format!(
            "OH_NativeDisplayManager_CaptureScreenPixelmap failed: {}",
            capture_code
        ));
    }

    let result = (|| {
        let mut info = std::ptr::null_mut();
        let create_info_code = unsafe { OH_PixelmapImageInfo_Create(&mut info) };
        if create_info_code != IMAGE_SUCCESS || info.is_null() {
            return Err(format!(
                "OH_PixelmapImageInfo_Create failed: {}",
                create_info_code
            ));
        }

        let image_result = (|| {
            let get_info_code = unsafe { OH_PixelmapNative_GetImageInfo(pixelmap, info) };
            if get_info_code != IMAGE_SUCCESS {
                return Err(format!(
                    "OH_PixelmapNative_GetImageInfo failed: {}",
                    get_info_code
                ));
            }
            let mut width = 0u32;
            let mut height = 0u32;
            let mut row_stride = 0u32;
            let mut pixel_format = 0i32;
            for (name, code) in [
                ("width", unsafe {
                    OH_PixelmapImageInfo_GetWidth(info, &mut width)
                }),
                ("height", unsafe {
                    OH_PixelmapImageInfo_GetHeight(info, &mut height)
                }),
                ("row stride", unsafe {
                    OH_PixelmapImageInfo_GetRowStride(info, &mut row_stride)
                }),
                ("pixel format", unsafe {
                    OH_PixelmapImageInfo_GetPixelFormat(info, &mut pixel_format)
                }),
            ] {
                if code != IMAGE_SUCCESS {
                    return Err(format!("Failed to read Pixelmap {}: {}", name, code));
                }
            }
            if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
                return Err(format!("Invalid Pixelmap size {}x{}", width, height));
            }
            if pixel_format != PIXEL_FORMAT_RGBA_8888 && pixel_format != PIXEL_FORMAT_BGRA_8888 {
                return Err(format!("Unsupported Pixelmap format {}", pixel_format));
            }
            let width = width as usize;
            let height = height as usize;
            let row_stride = row_stride as usize;
            let row_bytes = width
                .checked_mul(4)
                .ok_or_else(|| "Pixelmap row size overflow".to_string())?;
            let allocation_bytes = row_stride
                .checked_mul(height)
                .ok_or_else(|| "Pixelmap allocation size overflow".to_string())?;
            if row_stride < row_bytes || allocation_bytes > MAX_CONTROLLED_FRAME_BYTES {
                return Err(format!(
                    "Invalid Pixelmap stride/size: stride={} bytes={} max={}",
                    row_stride, allocation_bytes, MAX_CONTROLLED_FRAME_BYTES
                ));
            }
            let mut source = vec![0u8; allocation_bytes];
            let mut source_size = source.len();
            let read_code = unsafe {
                OH_PixelmapNative_ReadPixels(pixelmap, source.as_mut_ptr(), &mut source_size)
            };
            let required_source_bytes = row_stride
                .checked_mul(height.saturating_sub(1))
                .and_then(|value| value.checked_add(row_bytes))
                .ok_or_else(|| "Pixelmap readable size overflow".to_string())?;
            if read_code != IMAGE_SUCCESS || source_size < required_source_bytes {
                return Err(format!(
                    "OH_PixelmapNative_ReadPixels failed: code={} bytes={}/{}",
                    read_code, source_size, required_source_bytes
                ));
            }

            let tight_bytes = row_bytes
                .checked_mul(height)
                .ok_or_else(|| "Pixelmap frame size overflow".to_string())?;
            let mut rgba = vec![0u8; tight_bytes];
            for row in 0..height {
                let source_row = &source[row * row_stride..row * row_stride + row_bytes];
                let target_row = &mut rgba[row * row_bytes..(row + 1) * row_bytes];
                if pixel_format == PIXEL_FORMAT_RGBA_8888 {
                    target_row.copy_from_slice(source_row);
                } else {
                    for (source_pixel, target_pixel) in source_row
                        .chunks_exact(4)
                        .zip(target_row.chunks_exact_mut(4))
                    {
                        target_pixel.copy_from_slice(&[
                            source_pixel[2],
                            source_pixel[1],
                            source_pixel[0],
                            source_pixel[3],
                        ]);
                    }
                }
            }
            Ok((rgba, width, height))
        })();
        unsafe {
            let _ = OH_PixelmapImageInfo_Release(info);
        }
        image_result
    })();
    unsafe {
        let _ = OH_PixelmapNative_Release(pixelmap);
    }
    result
}

#[cfg(target_env = "ohos")]
fn resize_controlled_rgba(
    rgba: &[u8],
    source_width: usize,
    source_height: usize,
    target_width: usize,
    target_height: usize,
) -> Option<Vec<u8>> {
    let source_len = source_width.checked_mul(source_height)?.checked_mul(4)?;
    if source_width == 0
        || source_height == 0
        || target_width == 0
        || target_height == 0
        || rgba.len() != source_len
    {
        return None;
    }
    if source_width == target_width && source_height == target_height {
        return Some(rgba.to_vec());
    }
    let target_len = target_width.checked_mul(target_height)?.checked_mul(4)?;
    if target_len > MAX_CONTROLLED_FRAME_BYTES {
        return None;
    }
    let mut resized = vec![0u8; target_len];
    for target_y in 0..target_height {
        let source_y = target_y.saturating_mul(source_height) / target_height;
        for target_x in 0..target_width {
            let source_x = target_x.saturating_mul(source_width) / target_width;
            let source_offset = (source_y * source_width + source_x) * 4;
            let target_offset = (target_y * target_width + target_x) * 4;
            resized[target_offset..target_offset + 4]
                .copy_from_slice(&rgba[source_offset..source_offset + 4]);
        }
    }
    Some(resized)
}

fn controlled_pixelmap_stream_size(width: usize, height: usize) -> (usize, usize) {
    let longest = width.max(height);
    if longest <= CONTROLLED_PIXELMAP_MAX_DIMENSION {
        return (width, height);
    }
    let scaled_width = width
        .saturating_mul(CONTROLLED_PIXELMAP_MAX_DIMENSION)
        .checked_div(longest)
        .unwrap_or(width)
        .max(2)
        & !1;
    let scaled_height = height
        .saturating_mul(CONTROLLED_PIXELMAP_MAX_DIMENSION)
        .checked_div(longest)
        .unwrap_or(height)
        .max(2)
        & !1;
    (scaled_width.max(2), scaled_height.max(2))
}

#[cfg(target_env = "ohos")]
fn start_controlled_capture_fallback_watchdog(display_id: u32) {
    if CONTROLLED_CAPTURE_FALLBACK_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let spawn_result = std::thread::Builder::new()
        .name("ohos-screen-fallback".to_owned())
        .spawn(move || {
            std::thread::sleep(CONTROLLED_CAPTURE_FALLBACK_START_DELAY);
            let native_frames = controlled_runtime()
                .lock()
                .map(|state| state.native_capture_frames)
                .unwrap_or_default();
            if native_frames > 0 || !CONTROLLED_CAPTURE_FALLBACK_RUNNING.load(Ordering::Acquire) {
                CONTROLLED_CAPTURE_FALLBACK_RUNNING.store(false, Ordering::Release);
                return;
            }
            if let Ok(mut state) = controlled_runtime().lock() {
                state.screenshot_fallback_active = true;
            }
            log::warn!(
                "AVScreenCapture delivered no video frames; enabling display screenshot fallback"
            );
            while CONTROLLED_CAPTURE_FALLBACK_RUNNING.load(Ordering::Acquire)
                && CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) != 0
            {
                let started_at = Instant::now();
                match unsafe { capture_display_pixelmap_rgba(display_id) } {
                    Ok((rgba, width, height)) => {
                        if !CONTROLLED_CAPTURE_FALLBACK_RUNNING.load(Ordering::Acquire) {
                            break;
                        }
                        let (configured_width, configured_height) = ohos::host_screen_size();
                        let (frame, frame_width, frame_height) =
                            if configured_width > 0 && configured_height > 0 {
                                match resize_controlled_rgba(
                                    &rgba,
                                    width,
                                    height,
                                    configured_width,
                                    configured_height,
                                ) {
                                    Some(frame) => (frame, configured_width, configured_height),
                                    None => {
                                        if let Ok(mut state) = controlled_runtime().lock() {
                                            state.screenshot_fallback_errors =
                                                state.screenshot_fallback_errors.saturating_add(1);
                                            state.last_error = Some(format!(
                                                "Cannot normalize display frame {}x{} to {}x{}",
                                                width, height, configured_width, configured_height
                                            ));
                                        }
                                        continue;
                                    }
                                }
                            } else {
                                (rgba, width, height)
                            };
                        let forwarded =
                            ohos::push_host_screen_frame_rgba(&frame, frame_width, frame_height);
                        if let Ok(mut state) = controlled_runtime().lock() {
                            state.screenshot_fallback_frames =
                                state.screenshot_fallback_frames.saturating_add(1);
                            state.native_capture_frames =
                                state.native_capture_frames.saturating_add(1);
                            state.native_capture_bytes = state
                                .native_capture_bytes
                                .saturating_add(frame.len() as u64);
                            state.native_capture_last_timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|duration| duration.as_nanos().min(i64::MAX as u128) as i64)
                                .unwrap_or_default();
                            if !forwarded {
                                state.last_error = Some(
                                    "Core rejected display screenshot fallback frame".to_string(),
                                );
                            }
                        }
                    }
                    Err(err) => {
                        if let Ok(mut state) = controlled_runtime().lock() {
                            state.screenshot_fallback_errors =
                                state.screenshot_fallback_errors.saturating_add(1);
                            if state.screenshot_fallback_errors == 1
                                || state.screenshot_fallback_errors % 25 == 0
                            {
                                state.last_error = Some(err.clone());
                                log::error!("Display screenshot fallback failed: {}", err);
                            }
                        }
                    }
                }
                if let Some(remaining) =
                    CONTROLLED_CAPTURE_FALLBACK_FRAME_INTERVAL.checked_sub(started_at.elapsed())
                {
                    std::thread::sleep(remaining);
                }
            }
            CONTROLLED_CAPTURE_FALLBACK_RUNNING.store(false, Ordering::Release);
            if let Ok(mut state) = controlled_runtime().lock() {
                state.screenshot_fallback_active = false;
            }
        });
    match spawn_result {
        Ok(handle) => {
            if let Ok(mut slot) = controlled_capture_fallback_thread().lock() {
                *slot = Some(handle);
            }
        }
        Err(err) => {
            CONTROLLED_CAPTURE_FALLBACK_RUNNING.store(false, Ordering::Release);
            if let Ok(mut state) = controlled_runtime().lock() {
                state.last_error = Some(format!(
                    "Failed to start display screenshot fallback: {}",
                    err
                ));
            }
        }
    }
}

#[cfg(target_env = "ohos")]
unsafe extern "C" fn controlled_input_authorize_callback(status: i32) {
    CONTROLLED_INPUT_AUTH_STATUS.store(status, Ordering::Release);
}

#[derive(Clone, Copy, Default)]
struct SurfaceBinding {
    surface_id: Option<u64>,
    decode_size: Option<(usize, usize)>,
}

struct RenderStats {
    window_started_at: Instant,
    frames_in_window: usize,
    fps: f64,
    total_frames: usize,
    last_frame_at: Option<Instant>,
    total_decode_latency_ms: u64,
    decode_latency_samples: usize,
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            window_started_at: Instant::now(),
            frames_in_window: 0,
            fps: 0.0,
            total_frames: 0,
            last_frame_at: None,
            total_decode_latency_ms: 0,
            decode_latency_samples: 0,
        }
    }
}

impl RenderStats {
    fn record_frame(&mut self, latency: Option<u64>) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_started_at);
        if elapsed.as_secs_f64() >= 1.0 {
            self.fps = if self.frames_in_window > 0 {
                self.frames_in_window as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };
            self.frames_in_window = 0;
            self.window_started_at = now;
        }
        self.frames_in_window += 1;
        self.total_frames += 1;
        self.last_frame_at = Some(now);
        if let Some(latency) = latency {
            self.total_decode_latency_ms += latency;
            self.decode_latency_samples += 1;
        }
    }

    fn snapshot(&self) -> Value {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_started_at).as_secs_f64();
        let fps = if self.frames_in_window > 0 && elapsed > 0.0 {
            self.frames_in_window as f64 / elapsed
        } else {
            self.fps
        };
        let last_frame_age_ms = self
            .last_frame_at
            .map(|last_frame| now.duration_since(last_frame).as_millis() as u64)
            .unwrap_or(0);
        json!({
            "fps": if last_frame_age_ms > 2000 { 0.0 } else { fps },
            "totalFrames": self.total_frames,
            "lastFrameAgeMs": last_frame_age_ms,
            "hasRenderedFrame": self.last_frame_at.is_some(),
            "avgDecodeLatencyMs": if self.decode_latency_samples == 0 {
                0.0
            } else {
                self.total_decode_latency_ms as f64 / self.decode_latency_samples as f64
            },
        })
    }
}

#[derive(Clone)]
struct BridgeSession {
    session_id: String,
    core_session_id: Option<String>,
    peer_target: String,
    normalized_peer_id: String,
    custom_server: Option<String>,
    server_key: Option<String>,
    relay_suffix_requested: bool,
    force_relay: bool,
    conn_type: String,
    view_only: bool,
    phase: String,
    last_action: String,
    last_error: Option<String>,
    password_present: bool,
    shared_password: bool,
    password_ephemeral: bool,
    remember_requested: bool,
    two_factor_pending: bool,
    selected_displays: Vec<i32>,
    switch_uuid: Option<String>,
    conn_token_present: bool,
    last_pointer_payload: Option<String>,
    last_key_payload: Option<String>,
    last_text_payload: Option<String>,
}

struct NormalizedPeerTarget {
    peer_target: String,
    normalized_peer_id: String,
    custom_server: Option<String>,
    server_key: Option<String>,
    relay_suffix_requested: bool,
    effective_force_relay: bool,
}

fn surface_binding_store() -> &'static Mutex<HashMap<(String, u32), SurfaceBinding>> {
    SURFACE_BINDINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn core_event_queue_store() -> &'static Mutex<HashMap<String, VecDeque<Value>>> {
    CORE_EVENT_QUEUES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn render_stats_store() -> &'static Mutex<HashMap<String, HashMap<usize, RenderStats>>> {
    RENDER_STATS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn input_event_queue_store() -> &'static Mutex<VecDeque<Value>> {
    INPUT_EVENT_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn account_job_store() -> &'static Mutex<BackgroundJsonJob> {
    ACCOUNT_JOB.get_or_init(|| Mutex::new(BackgroundJsonJob::default()))
}

fn account_options_job_store() -> &'static Mutex<BackgroundJsonJob> {
    ACCOUNT_OPTIONS_JOB.get_or_init(|| Mutex::new(BackgroundJsonJob::default()))
}

fn address_book_job_store() -> &'static Mutex<BackgroundJsonJob> {
    ADDRESS_BOOK_JOB.get_or_init(|| Mutex::new(BackgroundJsonJob::default()))
}

fn account_challenge_store() -> &'static Mutex<Option<AccountChallenge>> {
    ACCOUNT_CHALLENGE.get_or_init(|| Mutex::new(None))
}

fn clear_input_events() {
    if let Ok(mut queue) = input_event_queue_store().lock() {
        queue.clear();
    }
}

fn take_input_events(limit: usize) -> Vec<Value> {
    let Ok(mut queue) = input_event_queue_store().lock() else {
        return Vec::new();
    };
    let take = if limit == 0 {
        queue.len()
    } else {
        limit.min(queue.len())
    };
    queue.drain(..take).collect()
}

#[cfg(target_env = "ohos")]
unsafe extern "C" fn on_intercepted_key_event(key_event: *const InputKeyEvent) {
    if key_event.is_null() || !INPUT_INTERCEPTOR_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let event = json!({
        "type": "key",
        "action": OH_Input_GetKeyEventAction(key_event),
        "keyCode": OH_Input_GetKeyEventKeyCode(key_event),
        "actionTime": OH_Input_GetKeyEventActionTime(key_event),
        "windowId": OH_Input_GetKeyEventWindowId(key_event),
        "displayId": OH_Input_GetKeyEventDisplayId(key_event),
    });
    if let Ok(mut queue) = input_event_queue_store().lock() {
        if queue.len() >= MAX_INPUT_EVENTS {
            queue.pop_front();
        }
        queue.push_back(event);
    }
}

#[cfg(target_env = "ohos")]
fn record_core_event(session_id: flutter_ffi::SessionID, event: flutter_ffi::EventToUI) {
    let event = match event {
        flutter_ffi::EventToUI::Event(raw) => match serde_json::from_str(&raw) {
            Ok(event) => event,
            Err(_) if raw == "close" => json!({ "name": "close" }),
            Err(_) => json!({ "name": "unknown", "raw": raw }),
        },
        flutter_ffi::EventToUI::Rgba(display) => json!({ "name": "rgba", "display": display }),
        flutter_ffi::EventToUI::Texture(display, gpu_texture) => {
            json!({ "name": "texture", "display": display, "gpuTexture": gpu_texture })
        }
    };
    update_bridge_session_from_event(session_id, &event);
    let mut queues = core_event_queue_store().lock().unwrap();
    let queue = queues.entry(session_id.to_string()).or_default();
    if queue.len() >= MAX_EVENTS_PER_SESSION {
        queue.pop_front();
    }
    queue.push_back(event);
}

#[cfg(target_env = "ohos")]
fn update_bridge_session_from_event(session_id: flutter_ffi::SessionID, event: &Value) {
    let name = event
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let core_session_id = session_id.to_string();
    let mut sessions = session_store().lock().unwrap();
    let Some(session) = sessions
        .values_mut()
        .find(|session| session.core_session_id.as_deref() == Some(core_session_id.as_str()))
    else {
        return;
    };
    match name {
        "connection_ready" => {
            session.phase = "transport_ready".to_string();
            session.last_error = None;
            session.two_factor_pending = false;
        }
        "peer_info" | "sync_peer_info" => {
            session.phase = "connected".to_string();
            session.last_error = None;
            session.two_factor_pending = false;
        }
        "close" => {
            session.phase = "closed".to_string();
            session.two_factor_pending = false;
        }
        "msgbox" => {
            let message_type = event
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let title = event
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let text = event
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if message_type == "input-2fa" {
                session.phase = "awaiting_authentication".to_string();
                session.two_factor_pending = true;
            } else if matches!(
                message_type,
                "input-password"
                    | "re-input-password"
                    | "session-login"
                    | "session-re-login"
                    | "session-login-password"
            ) {
                session.phase = "awaiting_authentication".to_string();
                session.two_factor_pending = false;
            } else if title == "Connection Error" || message_type.contains("error") {
                session.phase = "failed".to_string();
                session.two_factor_pending = false;
                session.last_error = Some(if title.is_empty() {
                    text.to_string()
                } else {
                    format!("{}: {}", title, text)
                });
            }
        }
        _ => {}
    }
}

#[cfg(target_env = "ohos")]
fn record_render_stats(session_id: String, display: usize, latency: Option<u64>) {
    render_stats_store()
        .lock()
        .unwrap()
        .entry(session_id)
        .or_default()
        .entry(display)
        .or_default()
        .record_frame(latency);
}

fn take_core_events(session_id: &str, limit: usize) -> Vec<Value> {
    let mut queues = core_event_queue_store().lock().unwrap();
    let Some(queue) = queues.get_mut(session_id) else {
        return Vec::new();
    };
    let take = if limit == 0 {
        queue.len()
    } else {
        limit.min(queue.len())
    };
    queue.drain(..take).collect()
}

fn render_stats(session_id: &str, display: usize) -> Value {
    let stats = render_stats_store()
        .lock()
        .unwrap()
        .get(session_id)
        .and_then(|per_display| per_display.get(&display))
        .map(RenderStats::snapshot)
        .unwrap_or_else(|| {
            json!({
                "fps": 0.0,
                "totalFrames": 0,
                "lastFrameAgeMs": 0,
                "hasRenderedFrame": false,
                "avgDecodeLatencyMs": 0.0,
            })
        });
    json!({ "display": display, "stats": stats })
}

fn clear_core_state(session_id: &str) {
    core_event_queue_store().lock().unwrap().remove(session_id);
    render_stats_store().lock().unwrap().remove(session_id);
}

#[cfg(target_env = "ohos")]
fn lookup_headless_surface(peer_id: &str, display: usize) -> Option<DirectRenderTarget> {
    let binding = surface_binding_store()
        .lock()
        .unwrap()
        .get(&(peer_id.to_string(), display as u32))
        .copied()?;
    let surface_id = binding.surface_id?;
    Some(DirectRenderTarget {
        surface_id: Some(surface_id),
        decode_size: binding.decode_size,
    })
}

#[cfg(target_env = "ohos")]
fn register_surface_lookup_once() {
    ohos::register_direct_render_target_lookup(lookup_headless_surface);
}

#[cfg(not(target_env = "ohos"))]
fn register_surface_lookup_once() {}

#[cfg(target_env = "ohos")]
fn register_core_callbacks() {
    ohos::register_session_event_callback(record_core_event);
    ohos::register_render_stats_callback(record_render_stats);
}

#[cfg(not(target_env = "ohos"))]
fn register_core_callbacks() {}

fn set_surface_binding(peer_id: &str, display: u32, surface_id: Option<u64>) {
    let mut bindings = surface_binding_store().lock().unwrap();
    let key = (peer_id.to_string(), display);
    bindings.entry(key).or_default().surface_id = surface_id;
}

fn set_surface_decode_size(peer_id: &str, display: u32, width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }
    let mut bindings = surface_binding_store().lock().unwrap();
    bindings
        .entry((peer_id.to_string(), display))
        .or_default()
        .decode_size = Some((width, height));
}

fn clear_surface_bindings_for_peer(peer_id: &str) {
    let mut bindings = surface_binding_store().lock().unwrap();
    bindings.retain(|(bound_peer_id, _), _| bound_peer_id != peer_id);
}

fn core_session_id_for(session_id: &str) -> Option<flutter_ffi::SessionID> {
    session_store()
        .lock()
        .unwrap()
        .get(session_id)
        .and_then(parse_core_session_id)
}

#[napi]
pub fn native_version() -> String {
    format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[napi]
pub fn build_marker() -> String {
    BUILD_MARKER.to_string()
}

#[napi]
pub fn runtime_get_decoder_capabilities() -> String {
    json!({
      "buildMarker": BUILD_MARKER,
      "capabilities": [
        query_decoder_capability("H264", MIME_VIDEO_AVC, ""),
        query_decoder_capability("H265", MIME_VIDEO_HEVC, ""),
        query_decoder_capability("AV1", MIME_VIDEO_AV1, "libaom"),
        query_decoder_capability("VP9", MIME_VIDEO_VP9, "libvpx"),
        query_decoder_capability("VP8", MIME_VIDEO_VP8, "libvpx"),
      ]
    })
    .to_string()
}

#[napi]
pub fn runtime_init(app_dir: String, device_name: String) -> String {
    init_hilog_logger_once();
    let normalized_device_name = if device_name.trim().is_empty() {
        "RustDesk HMOS".to_string()
    } else {
        device_name.trim().to_string()
    };
    register_surface_lookup_once();
    register_core_callbacks();
    flutter_ffi::main_device_name(normalized_device_name.clone());
    flutter_ffi::main_set_home_dir(app_dir.clone());
    flutter_ffi::main_init(app_dir.clone(), String::new());
    json!({
      "ok": true,
      "action": "runtime_init",
      "message": "RustDesk runtime initialized",
      "buildMarker": BUILD_MARKER,
      "appDir": app_dir,
      "deviceName": normalized_device_name,
      "apiServer": configured_api_server(),
      "upstream": upstream_status_value()
    })
    .to_string()
}

#[napi]
pub fn runtime_list_local_dir(path: String) -> String {
    let normalized_path = path.trim();
    if normalized_path.is_empty() {
        return json!({
          "ok": false,
          "action": "runtime_list_local_dir",
          "message": "Directory path is empty",
          "entries": []
        })
        .to_string();
    }

    match fs::read_dir(normalized_path) {
        Ok(entries) => {
            let mut names = entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            names.sort_unstable();
            json!({
              "ok": true,
              "action": "runtime_list_local_dir",
              "message": "Directory listed",
              "entries": names
            })
            .to_string()
        }
        Err(err) => json!({
          "ok": false,
          "action": "runtime_list_local_dir",
          "message": format!("Unable to list {}: {}", normalized_path, err),
          "entries": []
        })
        .to_string(),
    }
}

#[napi]
pub fn runtime_get_api_server() -> String {
    configured_api_server()
}

#[napi]
pub fn runtime_get_account_state() -> String {
    account_state_snapshot().to_string()
}

#[napi]
pub fn runtime_get_account_login_options() -> String {
    account_login_options().to_string()
}

#[napi]
pub fn runtime_start_account_login_options() -> String {
    start_background_job(
        account_options_job_store(),
        "runtime_start_account_login_options",
        |_generation| account_login_options(),
    )
}

#[napi]
pub fn runtime_poll_account_login_options() -> String {
    poll_background_job(
        account_options_job_store(),
        "runtime_poll_account_login_options",
    )
}

#[napi]
pub fn runtime_start_account_login(payload_json: String) -> String {
    let payload = match parse_json_payload(&payload_json, "account login") {
        Ok(payload) => payload,
        Err(message) => return job_start_error("runtime_start_account_login", message),
    };
    let username = json_field_string(&payload, "username").trim().to_string();
    let password = json_field_string(&payload, "password");
    if username.is_empty() || password.is_empty() {
        return job_start_error(
            "runtime_start_account_login",
            "Username and password are required".to_string(),
        );
    }
    start_account_login_job(username, password)
}

#[napi]
pub fn runtime_start_account_oidc(provider: String) -> String {
    let provider = provider.trim().to_string();
    if provider.is_empty() {
        return job_start_error(
            "runtime_start_account_oidc",
            "Account provider is required".to_string(),
        );
    }
    if configured_api_server().is_empty() {
        return job_start_error(
            "runtime_start_account_oidc",
            "Account API is not configured".to_string(),
        );
    }
    cancel_background_job(account_job_store());
    cancel_background_job(address_book_job_store());
    if let Ok(mut challenge) = account_challenge_store().lock() {
        *challenge = None;
    }
    flutter_ffi::main_account_auth(provider.clone(), true);
    json!({
      "ok": true,
      "action": "runtime_start_account_oidc",
      "state": "pending",
      "provider": provider
    })
    .to_string()
}

#[napi]
pub fn runtime_poll_account_oidc() -> String {
    if configured_api_server().is_empty() {
        flutter_ffi::main_account_auth_cancel();
        return background_job_failure(
            "runtime_poll_account_oidc",
            "server",
            "Account API is not configured".to_string(),
        )
        .to_string();
    }
    let raw = flutter_ffi::main_account_auth_result();
    let result = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}));
    let failed_message = json_field_string(&result, "failed_msg");
    if !failed_message.is_empty() {
        return background_job_failure("runtime_poll_account_oidc", "provider", failed_message)
            .to_string();
    }
    if let Some(auth_body) = result.get("auth_body").filter(|body| body.is_object()) {
        let response_type = json_field_string(auth_body, "type");
        if response_type == "access_token" {
            let user = auth_body
                .get("user")
                .filter(|user| user.is_object())
                .cloned()
                .unwrap_or_else(|| json!({}));
            return json!({
              "ok": true,
              "action": "runtime_poll_account_oidc",
              "state": "authenticated",
              "user": account_user_summary(&user)
            })
            .to_string();
        }
        return background_job_failure(
            "runtime_poll_account_oidc",
            "protocol",
            format!("Unsupported OIDC response type: {}", response_type),
        )
        .to_string();
    }
    json!({
      "ok": true,
      "action": "runtime_poll_account_oidc",
      "state": "pending",
      "stateMessage": json_field_string(&result, "state_msg"),
      "url": json_field_string(&result, "url")
    })
    .to_string()
}

#[napi]
pub fn runtime_cancel_account_oidc() -> String {
    flutter_ffi::main_account_auth_cancel();
    json!({
      "ok": true,
      "action": "runtime_cancel_account_oidc",
      "state": "cancelled"
    })
    .to_string()
}

#[napi]
pub fn runtime_start_account_verification(payload_json: String) -> String {
    let payload = match parse_json_payload(&payload_json, "account verification") {
        Ok(payload) => payload,
        Err(message) => {
            return job_start_error("runtime_start_account_verification", message);
        }
    };
    let challenge_id = json_field_string(&payload, "challengeId")
        .trim()
        .to_string();
    let code = json_field_string(&payload, "code").trim().to_string();
    if challenge_id.is_empty() || code.is_empty() {
        return job_start_error(
            "runtime_start_account_verification",
            "Challenge and verification code are required".to_string(),
        );
    }
    start_account_verification_job(challenge_id, code)
}

#[napi]
pub fn runtime_poll_account_action() -> String {
    poll_background_job(account_job_store(), "runtime_poll_account_action")
}

#[napi]
pub fn runtime_account_logout() -> String {
    account_logout()
}

#[napi]
pub fn runtime_cancel_account_challenge() -> String {
    match account_challenge_store().lock() {
        Ok(mut challenge) => {
            *challenge = None;
            json!({
              "ok": true,
              "action": "runtime_cancel_account_challenge",
              "state": "cancelled"
            })
            .to_string()
        }
        Err(_) => job_start_error(
            "runtime_cancel_account_challenge",
            "Account challenge state is unavailable".to_string(),
        ),
    }
}

#[napi]
pub fn runtime_start_address_book_sync() -> String {
    start_address_book_sync_job()
}

#[napi]
pub fn runtime_poll_address_book_sync() -> String {
    poll_background_job(address_book_job_store(), "runtime_poll_address_book_sync")
}

#[napi]
pub fn runtime_get_server_config() -> String {
    json!({
      "ok": true,
      "action": "runtime_get_server_config",
      "config": server_config_snapshot()
    })
    .to_string()
}

#[napi]
pub async fn runtime_test_server_config(config_json: String) -> String {
    let mut config = match parse_server_config(&config_json) {
        Ok(config) => config,
        Err(message) => {
            return json!({
              "ok": false,
              "action": "runtime_test_server_config",
              "message": message,
              "errors": {}
            })
            .to_string();
        }
    };
    let errors = test_server_config(&config).await;
    let ok = server_config_errors_empty(&errors);
    let has_server = server_config_has_endpoint(&config);
    if let Some(fields) = config.as_object_mut() {
        fields.insert("remoteReachable".to_string(), (ok && has_server).into());
        fields.insert(
            "effectiveApiServer".to_string(),
            configured_api_server().into(),
        );
        fields.insert(
            "resultState".to_string(),
            (if ok {
                if has_server {
                    "test-success"
                } else {
                    "empty-success"
                }
            } else {
                "error"
            })
            .into(),
        );
    }
    json!({
      "ok": ok,
      "action": "runtime_test_server_config",
      "message": if ok {
          if has_server {
              "已填写的服务器均返回了有效的 RustDesk 服务响应"
          } else {
              "未配置服务器；将使用局域网直连或手动直连"
          }
      } else {
          "服务器未通过 RustDesk 服务协议校验"
      },
      "config": config,
      "errors": errors
    })
    .to_string()
}

#[napi]
pub fn runtime_set_server_config(config_json: String) -> String {
    let config = match parse_server_config(&config_json) {
        Ok(config) => config,
        Err(message) => {
            return json!({
              "ok": false,
              "action": "runtime_set_server_config",
              "message": message,
              "errors": {}
            })
            .to_string();
        }
    };
    let errors = validate_server_config_format(&config);
    if !server_config_errors_empty(&errors) {
        return json!({
          "ok": false,
          "action": "runtime_set_server_config",
          "message": "服务器配置格式有误",
          "config": config,
          "errors": errors
        })
        .to_string();
    }

    let old_api_server = configured_api_server();
    flutter_ffi::main_set_option(
        "custom-rendezvous-server".to_string(),
        json_field_string(&config, "idServer"),
    );
    flutter_ffi::main_set_option(
        "relay-server".to_string(),
        json_field_string(&config, "relayServer"),
    );
    flutter_ffi::main_set_option(
        "api-server".to_string(),
        json_field_string(&config, "apiServer"),
    );
    flutter_ffi::main_set_option("key".to_string(), json_field_string(&config, "key"));
    let new_api_server = configured_api_server();
    let account_reset_required = old_api_server != new_api_server;
    if account_reset_required {
        cancel_background_job(account_job_store());
        cancel_background_job(account_options_job_store());
        cancel_background_job(address_book_job_store());
        flutter_ffi::main_account_auth_cancel();
        clear_account_state();
    }

    let mut saved_config = server_config_snapshot();
    if let Some(fields) = saved_config.as_object_mut() {
        fields.insert("remoteReachable".to_string(), false.into());
        fields.insert("resultState".to_string(), "save-success".into());
    }
    json!({
      "ok": true,
      "action": "runtime_set_server_config",
      "message": if server_config_has_endpoint(&config) {
          "服务器配置已保存；可点击“测试配置”检查连接"
      } else {
          "服务器配置已清空；当前不使用自定义服务器"
      },
      "config": saved_config,
      "errors": errors,
      "accountResetRequired": account_reset_required
    })
    .to_string()
}

#[napi]
pub fn runtime_list_recent_peers() -> String {
    let raw = flutter_ffi::main_load_recent_peers_for_ab("[]".to_string());
    match parse_json_list(&raw, "recent peer list") {
        Ok(peers) => {
            let peers = peers.iter().map(recent_peer_summary).collect::<Vec<_>>();
            json!({
              "ok": true,
              "action": "runtime_list_recent_peers",
              "count": peers.len(),
              "peers": peers
            })
            .to_string()
        }
        Err(message) => json!({
          "ok": false,
          "action": "runtime_list_recent_peers",
          "message": message,
          "count": 0,
          "peers": []
        })
        .to_string(),
    }
}

#[napi]
pub async fn runtime_query_peer_online_states(peer_ids_json: String) -> String {
    let values = match serde_json::from_str::<Value>(&peer_ids_json) {
        Ok(Value::Array(values)) => values,
        Ok(_) => {
            return json!({
              "ok": false,
              "action": "runtime_query_peer_online_states",
              "message": "Peer id list must be a JSON array",
              "onlines": [],
              "offlines": []
            })
            .to_string();
        }
        Err(error) => {
            return json!({
              "ok": false,
              "action": "runtime_query_peer_online_states",
              "message": format!("Invalid peer id list: {error}"),
              "onlines": [],
              "offlines": []
            })
            .to_string();
        }
    };
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for value in values {
        let Some(id) = value.as_str().map(str::trim) else {
            continue;
        };
        if !id.is_empty() && seen.insert(id.to_owned()) {
            ids.push(id.to_owned());
        }
    }
    if ids.is_empty() {
        return json!({
          "ok": true,
          "action": "runtime_query_peer_online_states",
          "message": "No peers to query",
          "onlines": [],
          "offlines": []
        })
        .to_string();
    }
    match librustdesk::query_online_states_result(ids).await {
        Ok((onlines, offlines)) => json!({
          "ok": true,
          "action": "runtime_query_peer_online_states",
          "message": "Peer online states updated",
          "onlines": onlines,
          "offlines": offlines
        })
        .to_string(),
        Err(error) => json!({
          "ok": false,
          "action": "runtime_query_peer_online_states",
          "message": error.to_string(),
          "onlines": [],
          "offlines": []
        })
        .to_string(),
    }
}

enum DirectPeerProbeOutcome {
    Online,
    Offline(String),
    Unknown(String),
}

async fn probe_direct_peer_api(target: String) -> DirectPeerProbeOutcome {
    let normalized = match validate_socket_target(&target, hbb_common::config::WS_RENDEZVOUS_PORT) {
        Ok(target) => target,
        Err(message) => return DirectPeerProbeOutcome::Unknown(message),
    };
    let mut stream = match connect_test_stream(normalized, false).await {
        Ok(stream) => stream,
        Err(message) => return DirectPeerProbeOutcome::Offline(message),
    };
    let Some(frame) = stream.next_timeout(1_500).await else {
        return DirectPeerProbeOutcome::Unknown(
            "端口可连接，但未收到 RustDesk 直连握手".to_string(),
        );
    };
    let bytes = match frame {
        Ok(bytes) => bytes,
        Err(error) => {
            return DirectPeerProbeOutcome::Unknown(format!("读取 RustDesk 直连握手失败: {error}"))
        }
    };
    let message = match PeerMessage::parse_from_bytes(&bytes) {
        Ok(message) => message,
        Err(_) => {
            return DirectPeerProbeOutcome::Unknown("端口响应不是 RustDesk 直连协议".to_string())
        }
    };
    if matches!(
        message.union,
        Some(peer_message::Union::Hash(_)) | Some(peer_message::Union::SignedId(_))
    ) {
        DirectPeerProbeOutcome::Online
    } else {
        DirectPeerProbeOutcome::Unknown("端口未返回 RustDesk 直连认证握手".to_string())
    }
}

#[napi]
pub async fn runtime_probe_peer_direct_states(targets_json: String) -> String {
    let values = match serde_json::from_str::<Value>(&targets_json) {
        Ok(Value::Array(values)) => values,
        Ok(_) => {
            return json!({
              "ok": false,
              "action": "runtime_probe_peer_direct_states",
              "message": "Direct peer target list must be a JSON array",
              "onlines": [],
              "offlines": [],
              "unknowns": []
            })
            .to_string();
        }
        Err(error) => {
            return json!({
              "ok": false,
              "action": "runtime_probe_peer_direct_states",
              "message": format!("Invalid direct peer target list: {error}"),
              "onlines": [],
              "offlines": [],
              "unknowns": []
            })
            .to_string();
        }
    };
    let mut seen = HashSet::new();
    let mut targets = Vec::new();
    for value in values.into_iter().take(64) {
        let key = json_field_string(&value, "key");
        let target = json_field_string(&value, "target");
        if !key.is_empty() && !target.is_empty() && seen.insert(key.clone()) {
            targets.push((key, target));
        }
    }
    if targets.is_empty() {
        return json!({
          "ok": true,
          "action": "runtime_probe_peer_direct_states",
          "message": "No direct peers to probe",
          "onlines": [],
          "offlines": [],
          "unknowns": []
        })
        .to_string();
    }

    let mut tasks = hbb_common::tokio::task::JoinSet::new();
    for (key, target) in targets {
        tasks.spawn(async move { (key, probe_direct_peer_api(target).await) });
    }
    let mut onlines = Vec::new();
    let mut offlines = Vec::new();
    let mut unknowns = Vec::new();
    let mut details = serde_json::Map::new();
    while let Some(result) = tasks.join_next().await {
        let Ok((key, outcome)) = result else {
            continue;
        };
        match outcome {
            DirectPeerProbeOutcome::Online => onlines.push(key),
            DirectPeerProbeOutcome::Offline(message) => {
                details.insert(key.clone(), Value::String(message));
                offlines.push(key);
            }
            DirectPeerProbeOutcome::Unknown(message) => {
                details.insert(key.clone(), Value::String(message));
                unknowns.push(key);
            }
        }
    }
    json!({
      "ok": true,
      "action": "runtime_probe_peer_direct_states",
      "message": "Direct peer states updated",
      "onlines": onlines,
      "offlines": offlines,
      "unknowns": unknowns,
      "details": details
    })
    .to_string()
}

#[napi]
pub fn runtime_remove_recent_peer(peer_id: String) -> String {
    let peer_id = peer_id.trim().to_string();
    if peer_id.is_empty() {
        return json!({
          "ok": false,
          "action": "runtime_remove_recent_peer",
          "message": "Peer id is empty"
        })
        .to_string();
    }
    let existed = flutter_ffi::main_peer_exists(peer_id.clone());
    flutter_ffi::main_remove_peer(peer_id.clone());
    let removed = !flutter_ffi::main_peer_exists(peer_id.clone());
    json!({
      "ok": removed,
      "action": "runtime_remove_recent_peer",
      "message": if removed { "Recent peer removed" } else { "Unable to remove recent peer" },
      "peerId": peer_id,
      "existed": existed,
      "removed": removed
    })
    .to_string()
}

#[napi]
pub fn runtime_list_favorites() -> String {
    let favorites = flutter_ffi::main_get_fav();
    json!({
      "ok": true,
      "action": "runtime_list_favorites",
      "count": favorites.len(),
      "favorites": favorites
    })
    .to_string()
}

#[napi]
pub fn runtime_set_favorite(peer_id: String, favorite: bool) -> String {
    let peer_id = peer_id.trim().to_string();
    if peer_id.is_empty() {
        return json!({
          "ok": false,
          "action": "runtime_set_favorite",
          "message": "Peer id is empty"
        })
        .to_string();
    }
    let mut favorites = flutter_ffi::main_get_fav();
    if favorite {
        if !favorites.contains(&peer_id) {
            favorites.push(peer_id.clone());
        }
    } else {
        favorites.retain(|id| id != &peer_id);
    }
    flutter_ffi::main_store_fav(favorites.clone());
    json!({
      "ok": true,
      "action": "runtime_set_favorite",
      "message": "Favorite state saved",
      "peerId": peer_id,
      "favorite": favorite,
      "favorites": favorites
    })
    .to_string()
}

#[napi]
pub fn runtime_scan_lan_peers() -> String {
    // A LAN address is volatile transport metadata. Start every explicit scan
    // from an empty snapshot so an offline device cannot keep presenting an
    // obsolete IP after DHCP/network changes.
    let previous_peers = hbb_common::config::LanPeers::load().peers;
    migrate_legacy_lan_passwords(&previous_peers);
    hbb_common::config::LanPeers::store(&[]);
    lan_preferred_ip_store().lock().unwrap().clear();
    let generation = LAN_SCAN_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let recent_raw = flutter_ffi::main_load_recent_peers_for_ab("[]".to_string());
    let recent = parse_json_list(&recent_raw, "recent LAN candidates").unwrap_or_default();
    flutter_ffi::main_discover();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(3_100));
        resolve_lan_preferred_ips(generation, &recent);
    });
    json!({
      "ok": true,
      "action": "runtime_scan_lan_peers",
      "message": "LAN discovery started",
      "pollAfterMs": 300,
      "timeoutMs": 4000
    })
    .to_string()
}

static LAN_SCAN_GENERATION: AtomicU64 = AtomicU64::new(0);

fn lan_preferred_ip_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn resolve_lan_preferred_ips(generation: u64, recent: &[Value]) {
    if LAN_SCAN_GENERATION.load(Ordering::Relaxed) != generation {
        return;
    }
    let peers = hbb_common::config::LanPeers::load().peers;
    let mut resolved = HashMap::new();
    for peer in peers {
        let mut discovered = peer.ip_mac.keys().cloned().collect::<Vec<_>>();
        discovered.sort_by(|left, right| right.cmp(left));
        discovered.dedup();
        let recent_candidate = recent_lan_candidate(&peer, recent);
        if let Some(candidate) = &recent_candidate {
            migrate_peer_password(&peer.id, candidate);
        }
        let mut candidates = Vec::new();
        if let Some(candidate) = recent_candidate {
            candidates.push(candidate);
        }
        for candidate in &discovered {
            if !candidates.contains(candidate) {
                candidates.push(candidate.clone());
            }
        }
        let preferred = candidates
            .iter()
            .find(|candidate| lan_direct_port_responds(candidate))
            .cloned()
            .or_else(|| discovered.first().cloned());
        if let Some(preferred) = preferred {
            resolved.insert(peer.id, preferred);
        }
    }
    if LAN_SCAN_GENERATION.load(Ordering::Relaxed) == generation {
        *lan_preferred_ip_store().lock().unwrap() = resolved;
    }
}

fn recent_lan_candidate(
    peer: &hbb_common::config::DiscoveryPeer,
    recent: &[Value],
) -> Option<String> {
    recent.iter().find_map(|row| {
        let candidate = json_field_string(row, "id");
        IpAddr::from_str(&candidate).ok()?;
        let recent_username = json_field_string(row, "username");
        let recent_hostname = json_field_string(row, "hostname");
        let username_match = !peer.username.is_empty() && peer.username == recent_username;
        let hostname_match = !peer.hostname.is_empty() && peer.hostname == recent_hostname;
        let same_identity = username_match || hostname_match;
        same_identity.then_some(candidate)
    })
}

fn lan_direct_port_responds(ip: &str) -> bool {
    let Ok(ip) = IpAddr::from_str(ip) else {
        return false;
    };
    let target = SocketAddr::new(ip, hbb_common::config::WS_RENDEZVOUS_PORT as u16);
    TcpStream::connect_timeout(&target, Duration::from_millis(250)).is_ok()
}

fn migrate_peer_password(peer_id: &str, legacy_target: &str) {
    if peer_id.is_empty() || legacy_target.is_empty() {
        return;
    }
    let mut stable_config = hbb_common::config::PeerConfig::load(peer_id);
    if !stable_config.password.is_empty() {
        return;
    }
    let legacy_config = hbb_common::config::PeerConfig::load(legacy_target);
    if !legacy_config.password.is_empty() {
        stable_config.password = legacy_config.password;
        stable_config.store(peer_id);
    }
}

fn migrate_legacy_lan_passwords(peers: &[hbb_common::config::DiscoveryPeer]) {
    for peer in peers {
        if peer.id.is_empty() {
            continue;
        }
        let mut stable_config = hbb_common::config::PeerConfig::load(&peer.id);
        if !stable_config.password.is_empty() {
            continue;
        }
        for ip in peer.ip_mac.keys() {
            let legacy_config = hbb_common::config::PeerConfig::load(ip);
            if !legacy_config.password.is_empty() {
                stable_config.password = legacy_config.password;
                stable_config.store(&peer.id);
                break;
            }
        }
    }
}

#[napi]
pub fn runtime_list_lan_peers() -> String {
    let raw =
        serde_json::to_string(&hbb_common::config::LanPeers::load().peers).unwrap_or_default();
    match parse_json_list(&raw, "LAN peer list") {
        Ok(peers) => {
            // Core inserts the newest discovery response first. Deduplicate by
            // stable RustDesk ID so an older persisted row can never win in UI
            // keyed rendering when username or address metadata changes.
            let mut seen_ids = HashSet::new();
            let recent_raw = flutter_ffi::main_load_recent_peers_for_ab("[]".to_string());
            let recent = parse_json_list(&recent_raw, "recent LAN candidates").unwrap_or_default();
            let peers = peers
                .iter()
                .filter(|peer| {
                    let id = json_field_string(peer, "id");
                    !id.is_empty() && seen_ids.insert(id)
                })
                .map(|peer| lan_peer_summary(peer, &recent))
                .collect::<Vec<_>>();
            json!({
              "ok": true,
              "action": "runtime_list_lan_peers",
              "count": peers.len(),
              "peers": peers
            })
            .to_string()
        }
        Err(message) => json!({
          "ok": false,
          "action": "runtime_list_lan_peers",
          "message": message,
          "count": 0,
          "peers": []
        })
        .to_string(),
    }
}

#[napi]
pub fn runtime_list_address_book_peers() -> String {
    let raw = flutter_ffi::main_load_ab();
    let cache = match serde_json::from_str::<Value>(&raw) {
        Ok(cache) => cache,
        Err(err) => {
            return json!({
              "ok": false,
              "action": "runtime_list_address_book_peers",
              "message": format!("Invalid address book cache: {}", err),
              "count": 0,
              "peers": [],
              "addressBooks": []
            })
            .to_string();
        }
    };
    let current_access_token = flutter_ffi::main_get_local_option("access_token".to_string()).0;
    let cached_access_token = json_field_string(&cache, "access_token");
    if current_access_token.is_empty() || cached_access_token != current_access_token {
        return json!({
          "ok": true,
          "action": "runtime_list_address_book_peers",
          "message": "Address book cache is unavailable for the current account",
          "count": 0,
          "peers": [],
          "addressBooks": []
        })
        .to_string();
    }
    let mut count = 0usize;
    let mut flat_peers = Vec::new();
    let address_books = cache
        .get("ab_entries")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    let guid = json_field_string(entry, "guid");
                    let name = json_field_string(entry, "name");
                    let peers = entry
                        .get("peers")
                        .and_then(Value::as_array)
                        .map(|peers| {
                            peers
                                .iter()
                                .map(address_book_peer_summary)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    count += peers.len();
                    flat_peers.extend(peers.iter().cloned().map(|mut peer| {
                        if let Value::Object(fields) = &mut peer {
                            fields.insert("addressBookGuid".to_string(), guid.clone().into());
                            fields.insert("addressBookName".to_string(), name.clone().into());
                        }
                        peer
                    }));
                    json!({
                      "guid": guid,
                      "name": name,
                      "tags": json_array_field(entry, "tags"),
                      "tagColors": json_field_string(entry, "tag_colors"),
                      "peers": peers
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
      "ok": true,
      "action": "runtime_list_address_book_peers",
      "count": count,
      "peers": flat_peers,
      "addressBooks": address_books
    })
    .to_string()
}

#[napi]
pub fn input_interceptor_start() -> String {
    clear_input_events();
    #[cfg(target_env = "ohos")]
    {
        let result = unsafe {
            OH_Input_AddKeyEventInterceptor(on_intercepted_key_event, std::ptr::null_mut())
        };
        let active = result == INPUT_SUCCESS || result == INPUT_REPEAT_INTERCEPTOR;
        INPUT_INTERCEPTOR_ACTIVE.store(active, Ordering::Release);
        return json!({
            "ok": active,
            "action": "input_interceptor_start",
            "message": if active {
                "Keyboard input interceptor is active"
            } else {
                "Unable to start keyboard input interceptor"
            },
            "active": active,
            "result": result,
        })
        .to_string();
    }
    #[cfg(not(target_env = "ohos"))]
    json!({
        "ok": false,
        "action": "input_interceptor_start",
        "message": "Input interception is only available on HarmonyOS",
        "active": false,
        "result": -1,
    })
    .to_string()
}

#[napi]
pub fn input_interceptor_stop() -> String {
    #[cfg(target_env = "ohos")]
    {
        let result = unsafe { OH_Input_RemoveKeyEventInterceptor() };
        let stopped = result == INPUT_SUCCESS;
        if stopped {
            INPUT_INTERCEPTOR_ACTIVE.store(false, Ordering::Release);
            clear_input_events();
        }
        return json!({
            "ok": stopped,
            "action": "input_interceptor_stop",
            "message": if stopped {
                "Keyboard input interceptor stopped"
            } else {
                "Unable to stop keyboard input interceptor"
            },
            "active": INPUT_INTERCEPTOR_ACTIVE.load(Ordering::Acquire),
            "result": result,
        })
        .to_string();
    }
    #[cfg(not(target_env = "ohos"))]
    json!({
        "ok": true,
        "action": "input_interceptor_stop",
        "message": "Input interceptor is not active",
        "active": false,
        "result": 0,
    })
    .to_string()
}

#[napi]
pub fn input_interceptor_poll_events(limit: u32) -> String {
    json!({
        "ok": true,
        "action": "input_interceptor_poll_events",
        "active": INPUT_INTERCEPTOR_ACTIVE.load(Ordering::Acquire),
        "events": take_input_events(limit as usize),
    })
    .to_string()
}

#[napi]
pub fn healthcheck() -> String {
    if core_binding_ready() {
        "rustdesk native har ready for real core binding".to_string()
    } else {
        format!("rustdesk native har scaffold ready: {CORE_BLOCKER_MESSAGE}")
    }
}

#[napi]
pub fn backend_summary() -> String {
    let upstream = upstream_status_value();
    json!({
      "integrationStage": if core_binding_ready() { "core_bindable" } else { "upstream_snapshot_present" },
      "upstreamRepo": RUSTDESK_UPSTREAM_REPO,
      "upstreamPath": upstream.get("rustdeskPath").cloned().unwrap_or(Value::Null),
      "nativeCrate": env!("CARGO_PKG_NAME"),
      "nativePackage": NATIVE_PACKAGE_NAME,
      "plannedModules": [
        "Flutter session bridge compatibility layer",
        "hbb_common protocol and config bridge",
        "HarmonyOS input, display, and frame adapters"
      ],
      "nextSteps": [
        "Consume polling events from ArkTS and map them into UI state.",
        "Render polled RGBA frames efficiently on the Harmony side.",
        "Replace intentional OHOS stubs only when those feature areas are needed."
      ],
      "notes": "The NAPI surface now calls real RustDesk session entry points. Remaining work is Harmony-side product integration for events and frame rendering.",
      "upstream": upstream
    })
    .to_string()
}

#[napi]
pub fn connection_flow_manifest() -> String {
    json!({
      "basis": "RustDesk Flutter session architecture",
      "upstream": upstream_status_value(),
      "chain": [
        {
          "step": 1,
          "name": "session_add",
          "summary": "UI creates a session object and seeds LoginConfigHandler.",
          "refs": [
            "third_party/rustdesk/src/flutter.rs:1295",
            "third_party/rustdesk/src/client.rs:1781"
          ]
        },
        {
          "step": 2,
          "name": "peer_target_normalization",
          "summary": "RustDesk parses id@server?... syntax and strips relay suffix /r or \\r.",
          "refs": [
            "third_party/rustdesk/src/client.rs:1792",
            "third_party/rustdesk/src/ui_interface.rs:1494"
          ]
        },
        {
          "step": 3,
          "name": "session_start",
          "summary": "UI starts the IO loop for the created session.",
          "refs": [
            "third_party/rustdesk/src/flutter.rs:1382",
            "third_party/rustdesk/src/client/io_loop.rs:168"
          ]
        },
        {
          "step": 4,
          "name": "rendezvous_bootstrap",
          "summary": "Client selects rendezvous server and sends PunchHoleRequest.",
          "refs": [
            "third_party/rustdesk/src/client.rs:292",
            "third_party/rustdesk/src/client.rs:460"
          ]
        },
        {
          "step": 5,
          "name": "direct_or_relay_transport",
          "summary": "Client handles PunchHoleResponse or RelayResponse and races direct/relay transport.",
          "refs": [
            "third_party/rustdesk/src/client.rs:486",
            "third_party/rustdesk/src/client.rs:534",
            "third_party/rustdesk/src/client.rs:633",
            "third_party/rustdesk/src/client.rs:837",
            "third_party/rustdesk/src/client.rs:901",
            "third_party/rustdesk/src/client.rs:758"
          ]
        },
        {
          "step": 6,
          "name": "hash_challenge",
          "summary": "After transport, the remote side sends a Hash challenge for login.",
          "refs": [
            "third_party/rustdesk/src/client/io_loop.rs:1320",
            "third_party/rustdesk/src/client.rs:3444"
          ]
        },
        {
          "step": 7,
          "name": "login_request",
          "summary": "Client hashes password with salt/challenge and sends login request.",
          "refs": [
            "third_party/rustdesk/src/ui_session_interface.rs:1360",
            "third_party/rustdesk/src/client.rs:3596",
            "third_party/rustdesk/src/client.rs:3620"
          ]
        },
        {
          "step": 8,
          "name": "two_factor_optional",
          "summary": "If required, the client submits 2FA before the peer is accepted.",
          "refs": [
            "third_party/rustdesk/src/ui_session_interface.rs:1370",
            "third_party/rustdesk/src/client/io_loop.rs:1325"
          ]
        },
        {
          "step": 9,
          "name": "connected_peer_info",
          "summary": "On success, RustDesk emits connection_ready and peer_info events and begins video/input handling.",
          "refs": [
            "third_party/rustdesk/src/client/io_loop.rs:184",
            "third_party/rustdesk/src/flutter.rs:735",
            "third_party/rustdesk/src/flutter.rs:893",
            "third_party/rustdesk/src/flutter_ffi.rs:2243"
          ]
        }
      ]
    })
    .to_string()
}

#[napi]
pub fn compile_blockers_manifest() -> String {
    json!({
      "upstream": upstream_status_value(),
      "blockers": compile_blockers(),
      "rootCauseSummary": [
        "The OHOS target triple reports target_os=linux and target_env=ohos.",
        "RustDesk now compiles as an OHOS-oriented client-only core in this workspace.",
        "The native HAR already forwards key session methods into real RustDesk session entry points.",
        "The remaining work is Harmony-native event delivery and decoded frame delivery back to ArkTS."
      ],
      "nextMoves": [
        "Add a NAPI event stream for connection_ready, peer_info, msgbox, and quality updates.",
        "Add a Harmony-native frame bridge for remote video output.",
        "Replace the remaining intentional OHOS stubs when those feature areas are needed."
      ]
    })
    .to_string()
}

#[napi]
pub fn session_api_manifest() -> String {
    json!({
      "designChoice": "Mirror RustDesk's Flutter session bridge rather than exposing raw Client internals.",
      "upstream": upstream_status_value(),
      "implementedScaffold": [
        {
          "name": "normalizePeerTarget",
          "refs": [
            "third_party/rustdesk/src/client.rs:1781",
            "third_party/rustdesk/src/ui_interface.rs:1494"
          ]
        },
        {
          "name": "sessionAdd",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:137",
            "third_party/rustdesk/src/flutter.rs:1295"
          ]
        },
        {
          "name": "sessionStart",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:178",
            "third_party/rustdesk/src/flutter.rs:1382"
          ]
        },
        {
          "name": "sessionLogin",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:232",
            "third_party/rustdesk/src/ui_session_interface.rs:1360"
          ]
        },
        {
          "name": "sessionSend2fa",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:244",
            "third_party/rustdesk/src/ui_session_interface.rs:1370"
          ]
        },
        {
          "name": "sessionReconnect",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:315",
            "third_party/rustdesk/src/ui_session_interface.rs:1278"
          ]
        },
        {
          "name": "sessionClose",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:263"
          ]
        },
        {
          "name": "sessionSendPointer",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:1863",
            "third_party/rustdesk/src/flutter.rs:1932"
          ]
        },
        {
          "name": "sessionSendMouse",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:1915",
            "third_party/rustdesk/src/ui_session_interface.rs:1211"
          ]
        },
        {
          "name": "sessionSetShowRemoteCursor",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:226",
            "third_party/rustdesk/src/flutter_ffi.rs:337"
          ]
        },
        {
          "name": "sessionSetCodecPreference",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:710",
            "third_party/rustdesk/libs/scrap/src/common/codec.rs:1013"
          ]
        },
        {
          "name": "sessionInputKey",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:637"
          ]
        },
        {
          "name": "sessionHandleFlutterKeyEvent",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:590",
            "third_party/rustdesk/src/ui_session_interface.rs:1038"
          ]
        },
        {
          "name": "sessionEnterOrLeave",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:634"
          ]
        },
        {
          "name": "sessionInputString",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:653"
          ]
        },
        {
          "name": "sessionSendChat",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:676"
          ]
        },
        {
          "name": "sessionPollEvents",
          "refs": [
            "third_party/rustdesk/src/flutter.rs:1767"
          ]
        },
        {
          "name": "sessionSwitchDisplay",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:564"
          ]
        },
        {
          "name": "sessionSetSize",
          "refs": [
            "third_party/rustdesk/src/flutter_ffi.rs:936"
          ]
        }
      ],
      "plannedLater": [
        {
          "name": "sessionGetRgbaSize",
          "refs": ["third_party/rustdesk/src/flutter_ffi.rs:2243"]
        },
        {
          "name": "sessionNextRgba",
          "refs": ["third_party/rustdesk/src/flutter_ffi.rs:2247"]
        }
      ],
      "plannedEvents": [
        "connection_ready",
        "peer_info",
        "msgbox",
        "permission",
        "update_quality_status",
        "Rgba"
      ],
      "note": "These NAPI methods now call through to real RustDesk sessions where possible, while event/frame consumption still uses Harmony-side polling."
    })
    .to_string()
}

#[napi]
pub fn normalize_peer_target(peer_target: String, force_relay: bool) -> String {
    let normalized = normalize_target(&peer_target, force_relay);
    json!({
      "peerTarget": normalized.peer_target,
      "normalizedPeerId": normalized.normalized_peer_id,
      "customServer": normalized.custom_server,
      "serverKey": normalized.server_key,
      "relaySuffixRequested": normalized.relay_suffix_requested,
      "forceRelay": normalized.effective_force_relay,
      "refs": [
        "third_party/rustdesk/src/client.rs:1781",
        "third_party/rustdesk/src/ui_interface.rs:1494"
      ]
    })
    .to_string()
}

#[napi]
pub fn session_add(session_id: String, peer_target: String, options_json: String) -> String {
    let options = match parse_json_payload(&options_json, "sessionAdd options") {
        Ok(value) => value,
        Err(message) => return action_response("session_add", false, message, None),
    };

    let resolved_session_id = make_session_id(&session_id);
    let core_session_id = make_core_session_id(&resolved_session_id);
    let force_relay = json_bool(&options, &["forceRelay", "force_relay"]);
    let normalized = normalize_target(&peer_target, force_relay);
    if !librustdesk::common::is_direct_ip_access(&normalized.normalized_peer_id)
        && normalized.custom_server.is_none()
        && configured_id_server().is_empty()
    {
        return action_response(
            "session_add",
            false,
            "Configure an ID server before connecting, or use an explicit id@server target"
                .to_string(),
            None,
        );
    }
    let conn_type = connection_type_from_options(&options).to_string();
    let view_only = conn_type == "default_conn"
        && json_bool(
            &options,
            &["isViewOnly", "is_view_only", "viewOnly", "view_only"],
        );

    let mut sessions = session_store().lock().unwrap();
    if sessions.contains_key(&resolved_session_id) {
        return action_response(
            "session_add",
            false,
            format!("Session {} already exists", resolved_session_id),
            sessions.get(&resolved_session_id),
        );
    }

    let password = json_string(&options, &["password"]).unwrap_or_default();
    let switch_uuid = json_string(&options, &["switchUuid", "switch_uuid"]);
    let conn_token = json_string(&options, &["connToken", "conn_token"]);
    let shared_password = json_bool(&options, &["isSharedPassword", "is_shared_password"]);
    let password_ephemeral = json_bool(&options, &["passwordEphemeral", "password_ephemeral"]);
    let is_file_transfer = conn_type == "file_transfer";
    let is_view_camera = conn_type == "view_camera";
    let is_terminal = conn_type == "terminal";
    let is_rdp = conn_type == "rdp";
    let is_port_forward = conn_type == "port_forward" || is_rdp;

    let core_session = match flutter::session_add(
        &core_session_id,
        &normalized.peer_target,
        is_file_transfer,
        is_view_camera,
        is_port_forward,
        is_rdp,
        is_terminal,
        switch_uuid.as_deref().unwrap_or_default(),
        normalized.effective_force_relay,
        password.clone(),
        shared_password,
        conn_token.clone(),
    ) {
        Ok(session) => session,
        Err(err) => {
            return action_response(
                "session_add",
                false,
                format!("RustDesk session_add failed: {}", err),
                None,
            );
        }
    };
    if password_ephemeral {
        core_session
            .lc
            .write()
            .unwrap()
            .set_password_ephemeral(true);
    }

    // `view-only` is a normal desktop session option rather than a separate
    // RustDesk connection type. Set it before the IO loop starts so the first
    // LoginRequest already disables keyboard, clipboard and file operations.
    // Always restore the requested value because RustDesk persists peer
    // options and a previous viewer session must not make a later control
    // session accidentally read-only.
    let current_view_only = flutter_ffi::session_get_toggle_option(
        core_session_id.clone(),
        VIEW_ONLY_OPTION.to_string(),
    )
    .unwrap_or(false);
    if current_view_only != view_only {
        flutter_ffi::session_toggle_option(core_session_id.clone(), VIEW_ONLY_OPTION.to_string());
    }

    let session = BridgeSession {
        session_id: resolved_session_id.clone(),
        core_session_id: Some(core_session_id.to_string()),
        peer_target: normalized.peer_target,
        normalized_peer_id: normalized.normalized_peer_id,
        custom_server: normalized.custom_server,
        server_key: normalized.server_key,
        relay_suffix_requested: normalized.relay_suffix_requested,
        force_relay: normalized.effective_force_relay,
        conn_type,
        view_only,
        phase: "created".to_string(),
        last_action: "session_add".to_string(),
        last_error: None,
        password_present: !password.is_empty(),
        shared_password,
        password_ephemeral,
        remember_requested: false,
        two_factor_pending: false,
        selected_displays: Vec::new(),
        switch_uuid,
        conn_token_present: conn_token.is_some(),
        last_pointer_payload: None,
        last_key_payload: None,
        last_text_payload: None,
    };

    sessions.insert(resolved_session_id.clone(), session);
    action_response(
        "session_add",
        true,
        format!(
            "Prepared RustDesk session {} with core session {}",
            resolved_session_id, core_session_id
        ),
        sessions.get(&resolved_session_id),
    )
}

#[napi]
pub fn session_start(session_id: String) -> String {
    update_session(&session_id, "session_start", |session| {
        if let Some(blocker) = core_binding_blocker() {
            session.phase = "blocked_transport_bootstrap".to_string();
            session.last_error = Some(blocker.clone());
            return (false, blocker);
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            session.phase = "invalid_core_session_id".to_string();
            session.last_error = Some("Missing core session id".to_string());
            return (false, "Missing core session id".to_string());
        };
        register_core_callbacks();
        match ohos::session_start_with_polling_events(&core_session_id, &session.peer_target) {
            Ok(()) => {
                session.phase = "transport_bootstrap_started".to_string();
                session.last_error = None;
                (true, "Started RustDesk headless IO loop".to_string())
            }
            Err(err) => {
                let message = format!("RustDesk session_start failed: {}", err);
                session.phase = "start_failed".to_string();
                session.last_error = Some(message.clone());
                (false, message)
            }
        }
    })
}

/// Set the desktop viewer policy for an existing session.
///
/// The flag is deliberately exposed as a setter instead of making callers
/// toggle the generic RustDesk option. This keeps the bridge state and the
/// persisted peer option in sync, and gives the bridge a single place to
/// enforce the no-input policy below.
#[napi]
pub fn session_set_view_only(session_id: String, enabled: bool) -> String {
    update_session(&session_id, "session_set_view_only", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.conn_type != "default_conn" {
            return (
                false,
                "View-only is only available for remote desktop sessions".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        let current = flutter_ffi::session_get_toggle_option(
            core_session_id.clone(),
            VIEW_ONLY_OPTION.to_string(),
        )
        .unwrap_or(false);
        if current != enabled {
            flutter_ffi::session_toggle_option(core_session_id, VIEW_ONLY_OPTION.to_string());
        }
        session.view_only = enabled;
        session.last_error = None;
        (
            true,
            if enabled {
                "Remote desktop viewer mode enabled".to_string()
            } else {
                "Remote desktop control mode enabled".to_string()
            },
        )
    })
}

#[napi]
pub fn session_login(session_id: String, login_json: String) -> String {
    let login = match parse_json_payload(&login_json, "sessionLogin payload") {
        Ok(value) => value,
        Err(message) => return action_response("session_login", false, message, None),
    };

    update_session(&session_id, "session_login", |session| {
        let password = json_string(&login, &["password"]).unwrap_or_default();
        let os_username = json_string(&login, &["osUsername", "os_username"]).unwrap_or_default();
        let os_password = json_string(&login, &["osPassword", "os_password"]).unwrap_or_default();
        let remember = json_bool(&login, &["remember"]) && !session.password_ephemeral;
        session.password_present = !password.is_empty();
        session.remember_requested = remember;
        // Password authentication is the first factor. The peer explicitly
        // requests Auth2FA later with an `input-2fa` event.
        session.two_factor_pending = false;

        if let Some(blocker) = core_binding_blocker() {
            session.phase = "blocked_login_handshake".to_string();
            session.last_error = Some(blocker.clone());
            return (false, blocker);
        }

        let Some(core_session_id) = parse_core_session_id(session) else {
            session.phase = "invalid_core_session_id".to_string();
            session.last_error = Some("Missing core session id".to_string());
            return (false, "Missing core session id".to_string());
        };

        flutter_ffi::session_login(
            core_session_id,
            os_username,
            os_password,
            password,
            remember,
        );
        session.phase = "login_submitted".to_string();
        session.last_error = None;
        (true, "Submitted RustDesk login request".to_string())
    })
}

#[napi]
pub fn session_send2fa(session_id: String, code: String, trust_this_device: bool) -> String {
    update_session(&session_id, "session_send2fa", |session| {
        if let Some(blocker) = core_binding_blocker() {
            session.phase = "blocked_two_factor".to_string();
            session.last_error = Some(blocker.clone());
            return (
                false,
                format!("{} Submitted code length: {}", blocker, code.len()),
            );
        }

        let Some(core_session_id) = parse_core_session_id(session) else {
            session.phase = "invalid_core_session_id".to_string();
            session.last_error = Some("Missing core session id".to_string());
            return (false, "Missing core session id".to_string());
        };

        if !session.two_factor_pending {
            session.phase = "awaiting_primary_authentication".to_string();
            session.last_error =
                Some("The peer has not requested 2FA; submit the password first".to_string());
            log::warn!(
                "Blocked premature 2FA submission: bridge_session={} core_session={} phase={}",
                session.session_id,
                core_session_id,
                session.phase
            );
            return (
                false,
                "The peer has not requested 2FA; submit the password first".to_string(),
            );
        }

        log::info!(
            "Forwarding Auth2FA: bridge_session={} core_session={} code_len={} trust_this_device={}",
            session.session_id,
            core_session_id,
            code.len(),
            trust_this_device
        );
        flutter_ffi::session_send2fa(core_session_id, code.clone(), trust_this_device);
        session.two_factor_pending = false;
        session.phase = "two_factor_submitted".to_string();
        session.last_error = None;
        (
            true,
            format!("Submitted RustDesk 2FA code (len={})", code.len()),
        )
    })
}

#[napi]
pub fn session_reconnect(session_id: String, force_relay: bool) -> String {
    update_session(&session_id, "session_reconnect", |session| {
        if force_relay {
            session.force_relay = true;
        }

        if let Some(blocker) = core_binding_blocker() {
            session.phase = "blocked_reconnect".to_string();
            session.last_error = Some(blocker.clone());
            return (false, blocker);
        }

        let Some(core_session_id) = parse_core_session_id(session) else {
            session.phase = "invalid_core_session_id".to_string();
            session.last_error = Some("Missing core session id".to_string());
            return (false, "Missing core session id".to_string());
        };

        if let Some(core_session) = flutter::sessions::get_session_by_session_id(&core_session_id) {
            core_session.reconnect(force_relay);
            session.phase = "reconnect_requested".to_string();
            session.last_error = None;
            (true, "Requested RustDesk reconnect".to_string())
        } else {
            let message = "RustDesk core session not found".to_string();
            session.phase = "reconnect_failed".to_string();
            session.last_error = Some(message.clone());
            (false, message)
        }
    })
}

#[napi]
pub fn session_close(session_id: String) -> String {
    let (core_session_id, normalized_peer_id, clear_surfaces) = {
        let mut sessions = session_store().lock().unwrap();
        let Some(session) = sessions.get_mut(&session_id) else {
            return action_response(
                "session_close",
                false,
                format!("Session {} was not found", session_id),
                None,
            );
        };
        session.last_action = "session_close".to_string();
        if session.phase == "closed" || session.phase == "closing" {
            return action_response(
                "session_close",
                true,
                "RustDesk session is already closing or closed".to_string(),
                Some(session),
            );
        }
        session.phase = "closing".to_string();
        session.two_factor_pending = false;
        session.last_error = None;
        (
            parse_core_session_id(session),
            session.normalized_peer_id.clone(),
            session.conn_type == "default_conn" || session.conn_type == "view_camera",
        )
    };

    if clear_surfaces {
        clear_surface_bindings_for_peer(&normalized_peer_id);
    }
    if let Some(core_session_id) = core_session_id {
        let core_session_key = core_session_id.to_string();
        flutter_ffi::session_close(core_session_id);
        clear_core_state(&core_session_key);
    }

    let mut sessions = session_store().lock().unwrap();
    let Some(session) = sessions.get_mut(&session_id) else {
        return action_response(
            "session_close",
            true,
            "Closed RustDesk session".to_string(),
            None,
        );
    };
    session.phase = "closed".to_string();
    session.two_factor_pending = false;
    session.last_error = None;
    action_response(
        "session_close",
        true,
        "Closed RustDesk session".to_string(),
        Some(session),
    )
}

#[napi]
pub fn session_send_pointer(session_id: String, pointer_json: String) -> String {
    update_session(&session_id, "session_send_pointer", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Input is disabled for a view-only session".to_string(),
            );
        }
        session.last_pointer_payload = Some(pointer_json.clone());
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_send_pointer(core_session_id, pointer_json.clone());
        session.last_error = None;
        (true, "Forwarded pointer payload to RustDesk".to_string())
    })
}

#[napi]
pub fn session_send_mouse(session_id: String, mouse_json: String) -> String {
    update_session(&session_id, "session_send_mouse", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Input is disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_send_mouse(core_session_id, mouse_json);
        session.last_error = None;
        (true, "Forwarded mouse payload to RustDesk".to_string())
    })
}

#[napi]
pub fn session_set_show_remote_cursor(session_id: String, enabled: bool) -> String {
    update_session(&session_id, "session_set_show_remote_cursor", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        let current = flutter_ffi::session_get_toggle_option(
            core_session_id.clone(),
            SHOW_REMOTE_CURSOR_OPTION.to_string(),
        )
        .unwrap_or(false);
        if current != enabled {
            flutter_ffi::session_toggle_option(
                core_session_id,
                SHOW_REMOTE_CURSOR_OPTION.to_string(),
            );
        }
        session.last_error = None;
        (
            true,
            format!(
                "Remote cursor visibility {} (forced, cached current={})",
                if enabled { "enabled" } else { "disabled" },
                current
            ),
        )
    })
}

#[napi]
pub fn session_set_codec_preference(session_id: String, codec: String) -> String {
    update_session(&session_id, "session_set_codec_preference", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        let normalized = match codec.trim().to_ascii_lowercase().as_str() {
            "h264" => "h264",
            "h265" => "h265",
            "vp8" => "vp8",
            "vp9" => "vp9",
            "av1" => "av1",
            "auto" | "" => "auto",
            other => {
                return (false, format!("Unsupported codec preference '{}'", other));
            }
        }
        .to_string();
        flutter_ffi::session_peer_option(
            core_session_id.clone(),
            CODEC_PREFERENCE_OPTION.to_string(),
            normalized.clone(),
        );
        // Before session_start the login request has not been created yet. The
        // persisted peer option is enough to make the first request advertise
        // the right decoder preference. Do not queue a standalone Misc before
        // the IO loop starts; once the session is live, push the update so a
        // user change can renegotiate an active stream.
        let should_push_update = session.phase != "created";
        if should_push_update {
            flutter_ffi::session_change_prefer_codec(core_session_id);
        }
        session.last_error = None;
        (
            true,
            if should_push_update {
                format!("Updated RustDesk codec preference to {}", normalized)
            } else {
                format!(
                    "Prepared RustDesk codec preference for first login: {}",
                    normalized
                )
            },
        )
    })
}

#[napi]
pub fn session_input_key(session_id: String, key_json: String) -> String {
    update_session(&session_id, "session_input_key", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Input is disabled for a view-only session".to_string(),
            );
        }
        session.last_key_payload = Some(key_json.clone());
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        if flutter::sessions::get_session_by_session_id(&core_session_id).is_none() {
            let message = "RustDesk core session not found".to_string();
            session.last_error = Some(message.clone());
            return (false, message);
        }
        let key = match parse_json_payload(&key_json, "sessionInputKey payload") {
            Ok(value) => value,
            Err(message) => return (false, message),
        };
        let name = json_string(&key, &["name"]).unwrap_or_default();
        let down = json_bool(&key, &["down"]);
        let press = json_bool(&key, &["press"]);
        let alt = json_bool(&key, &["alt"]);
        let ctrl = json_bool(&key, &["ctrl"]);
        let shift = json_bool(&key, &["shift"]);
        let command = json_bool(&key, &["command", "meta"]);
        log::info!(
            "[keyboard] legacy session={} core={} name={} down={} press={} alt={} ctrl={} shift={} command={}",
            session_id,
            core_session_id,
            name,
            down,
            press,
            alt,
            ctrl,
            shift,
            command
        );
        flutter_ffi::session_input_key(
            core_session_id,
            name,
            down,
            press,
            alt,
            ctrl,
            shift,
            command,
        );
        session.last_error = None;
        (true, "Forwarded key input to RustDesk".to_string())
    })
}

#[napi]
pub fn session_handle_flutter_key_event(session_id: String, key_json: String) -> String {
    update_session(&session_id, "session_handle_flutter_key_event", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Input is disabled for a view-only session".to_string(),
            );
        }
        session.last_key_payload = Some(key_json.clone());
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        let key = match parse_json_payload(&key_json, "sessionHandleFlutterKeyEvent payload") {
            Ok(value) => value,
            Err(message) => return (false, message),
        };
        let character = json_raw_string(&key, &["character", "name"]).unwrap_or_default();
        let Some(usb_hid) = json_i32(&key, &["usb_hid", "usbHid"]) else {
            return (
                false,
                "Missing usb_hid in sessionHandleFlutterKeyEvent payload".to_string(),
            );
        };
        let lock_modes = json_i32(&key, &["lock_modes", "lockModes"]).unwrap_or_default();
        let down_or_up = json_bool(&key, &["down_or_up", "downOrUp", "down"]);
        log::info!(
            "[keyboard] flutter session={} core={} usb_hid={} lock_modes={} down={} char_len={}",
            session_id,
            core_session_id,
            usb_hid,
            lock_modes,
            down_or_up,
            character.chars().count()
        );
        flutter_ffi::session_handle_flutter_key_event(
            core_session_id,
            character,
            usb_hid,
            lock_modes,
            down_or_up,
        );
        session.last_error = None;
        (true, "Forwarded Flutter key input to RustDesk".to_string())
    })
}

#[napi]
pub fn session_enter_or_leave(session_id: String, enter: bool) -> String {
    update_session(&session_id, "session_enter_or_leave", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Keyboard capture is disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        log::info!(
            "[keyboard] enter_or_leave session={} core={} enter={}",
            session_id,
            core_session_id,
            enter
        );
        flutter_ffi::session_enter_or_leave(core_session_id, enter);
        session.last_error = None;
        (
            true,
            if enter {
                "Entered RustDesk keyboard session".to_string()
            } else {
                "Left RustDesk keyboard session".to_string()
            },
        )
    })
}

#[napi]
pub fn session_input_string(session_id: String, value: String) -> String {
    update_session(&session_id, "session_input_string", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Input is disabled for a view-only session".to_string(),
            );
        }
        session.last_text_payload = Some(value.clone());
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_input_string(core_session_id, value.clone());
        session.last_error = None;
        (true, "Forwarded text input to RustDesk".to_string())
    })
}

#[napi]
pub fn session_send_clipboard(session_id: String, content: String) -> String {
    update_session(&session_id, "session_send_clipboard", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Clipboard is disabled for a view-only session".to_string(),
            );
        }
        if session.conn_type != "default_conn" {
            return (
                false,
                "Clipboard is only available for remote-control sessions".to_string(),
            );
        }
        let Some(_core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        #[cfg(target_env = "ohos")]
        {
            if ohos::update_client_text_clipboard(content.clone()) {
                session.last_error = None;
                (true, "Queued text clipboard for RustDesk".to_string())
            } else {
                let message =
                    "Clipboard synchronization is disabled by the current session permissions"
                        .to_string();
                session.last_error = Some(message.clone());
                (false, message)
            }
        }
        #[cfg(not(target_env = "ohos"))]
        {
            let _ = (&_core_session_id, &content);
            (
                false,
                "Clipboard bridge is only available on HarmonyOS".to_string(),
            )
        }
    })
}

#[napi]
pub fn session_send_chat(session_id: String, text: String) -> String {
    update_session(&session_id, "session_send_chat", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Chat is disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_send_chat(core_session_id, text.clone());
        session.last_error = None;
        (true, "Forwarded chat message to RustDesk".to_string())
    })
}

#[napi]
pub fn session_send_note(session_id: String, note: String) {
    let _ = update_session(&session_id, "session_send_note", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Notes are disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_send_note(core_session_id, note.clone());
        session.last_error = None;
        (true, "Forwarded session note to RustDesk".to_string())
    });
}

#[napi]
pub fn session_poll_events(session_id: String, limit: u32) -> String {
    let sessions = session_store().lock().unwrap();
    let Some(session) = sessions.get(&session_id) else {
        return action_response(
            "session_poll_events",
            false,
            format!("Session {} was not found", session_id),
            None,
        );
    };
    let Some(core_session_id) = parse_core_session_id(session) else {
        return action_response(
            "session_poll_events",
            false,
            "Missing core session id".to_string(),
            Some(session),
        );
    };
    let events = take_core_events(&core_session_id.to_string(), limit as usize);
    json!({
      "ok": true,
      "action": "session_poll_events",
      "events": events,
      "session": session_value(session),
      "upstream": upstream_status_value()
    })
    .to_string()
}

#[napi]
pub fn session_switch_display(session_id: String, displays_json: String) -> String {
    let parsed = match parse_json_payload(&displays_json, "sessionSwitchDisplay payload") {
        Ok(value) => value,
        Err(message) => return action_response("session_switch_display", false, message, None),
    };
    let displays = parse_display_list(&parsed);

    update_session(&session_id, "session_switch_display", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        session.selected_displays = displays.clone();
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_switch_display(false, core_session_id, displays.clone());
        session.last_error = None;
        (true, "Requested RustDesk display switch".to_string())
    })
}

#[napi]
pub fn session_set_size(session_id: String, display: u32, width: u32, height: u32) -> String {
    update_session(&session_id, "session_set_size", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        set_surface_decode_size(
            &session.normalized_peer_id,
            display,
            width as usize,
            height as usize,
        );
        flutter_ffi::session_set_size(
            core_session_id,
            display as usize,
            width as usize,
            height as usize,
        );
        session.last_error = None;
        (
            true,
            format!(
                "Updated RustDesk display {} size to {}x{}",
                display, width, height
            ),
        )
    })
}

#[napi]
pub fn session_change_resolution(
    session_id: String,
    display: u32,
    width: u32,
    height: u32,
) -> String {
    update_session(&session_id, "session_change_resolution", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "Remote resolution changes are disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_change_resolution(
            core_session_id,
            display as i32,
            width as i32,
            height as i32,
        );
        session.last_error = None;
        (
            true,
            format!(
                "Requested RustDesk display {} resolution change to {}x{}",
                display, width, height
            ),
        )
    })
}

#[napi]
pub fn session_set_image_quality(session_id: String, value: String) -> String {
    update_session(&session_id, "session_set_image_quality", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_set_image_quality(core_session_id, value.clone());
        session.last_error = None;
        (true, format!("Updated RustDesk image quality to {}", value))
    })
}

#[napi]
pub fn session_set_custom_fps(session_id: String, fps: u32) -> String {
    update_session(&session_id, "session_set_custom_fps", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_set_custom_fps(core_session_id, fps as i32);
        session.last_error = None;
        (true, format!("Updated RustDesk custom fps to {}", fps))
    })
}

#[napi]
pub fn session_get_image_quality(session_id: String) -> Option<String> {
    core_session_id_for(&session_id).and_then(flutter_ffi::session_get_image_quality)
}

#[napi]
pub fn session_get_conn_token(session_id: String) -> Option<String> {
    core_session_id_for(&session_id)
        .and_then(|core_session_id| flutter_ffi::session_get_conn_token(core_session_id).0)
}

#[napi]
pub fn session_get_enable_trusted_devices(session_id: String) -> bool {
    core_session_id_for(&session_id)
        .map(|core_session_id| flutter_ffi::session_get_enable_trusted_devices(core_session_id).0)
        .unwrap_or(false)
}

#[napi]
pub fn session_set_common(session_id: String, key: String, value: String) {
    let _ = update_session(&session_id, "session_set_common", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_set_common(core_session_id, key.clone(), value.clone());
        session.last_error = None;
        (
            true,
            format!("Updated RustDesk session common value {}", key),
        )
    });
}

#[napi]
pub fn session_set_custom_image_quality(session_id: String, value: i32) {
    let _ = update_session(&session_id, "session_set_custom_image_quality", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_set_custom_image_quality(core_session_id, value);
        session.last_error = None;
        (
            true,
            format!("Updated RustDesk custom image quality to {}", value),
        )
    });
}

#[napi]
pub fn session_get_toggle_option(session_id: String, option: String) -> bool {
    core_session_id_for(&session_id)
        .and_then(|core_session_id| flutter_ffi::session_get_toggle_option(core_session_id, option))
        .unwrap_or(false)
}

#[napi]
pub fn session_toggle_option(session_id: String, option: String) {
    let _ = update_session(&session_id, "session_toggle_option", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if option == VIEW_ONLY_OPTION {
            return (
                false,
                "Use session_set_view_only so the bridge policy stays synchronized".to_string(),
            );
        }
        if session.view_only {
            return (
                false,
                "Generic option changes are disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_toggle_option(core_session_id, option.clone());
        session.last_error = None;
        (true, format!("Toggled RustDesk option {}", option))
    });
}

#[napi]
pub fn session_peer_option(session_id: String, name: String, value: String) {
    let _ = update_session(&session_id, "session_peer_option", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if name == VIEW_ONLY_OPTION {
            return (
                false,
                "Use session_set_view_only so the bridge policy stays synchronized".to_string(),
            );
        }
        if session.view_only {
            return (
                false,
                "Peer option changes are disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_peer_option(core_session_id, name.clone(), value.clone());
        session.last_error = None;
        (true, format!("Updated RustDesk peer option {}", name))
    });
}

#[napi]
pub fn session_alternative_codecs(session_id: String) -> String {
    core_session_id_for(&session_id)
        .map(flutter_ffi::session_alternative_codecs)
        .unwrap_or_default()
}

#[napi]
pub fn session_on_waiting_for_image_dialog_show(session_id: String) -> String {
    update_session(
        &session_id,
        "session_on_waiting_for_image_dialog_show",
        |session| {
            if session.phase == "closed" {
                return (false, "Session is closed".to_string());
            }
            let Some(core_session_id) = parse_core_session_id(session) else {
                return (false, "Missing core session id".to_string());
            };
            flutter_ffi::session_on_waiting_for_image_dialog_show(core_session_id);
            session.last_error = None;
            (true, "Forwarded waiting-for-image notification".to_string())
        },
    )
}

#[napi]
pub fn session_get_remote_audio_state(session_id: String) -> String {
    let Some(core_session_id) = core_session_id_for(&session_id) else {
        return json!({
          "ok": false,
          "action": "session_get_remote_audio_state",
          "audio": {
            "available": false,
            "muted": false,
            "rendererActive": false,
            "errorText": "Session is unavailable"
          }
        })
        .to_string();
    };
    let muted =
        flutter_ffi::session_get_toggle_option(core_session_id, "disable-audio".to_string())
            .unwrap_or(false);
    json!({
      "ok": true,
      "action": "session_get_remote_audio_state",
      "audio": {
        "available": true,
        "muted": muted,
        "rendererActive": !muted,
        "errorText": ""
      }
    })
    .to_string()
}

#[napi]
pub fn session_read_remote_dir(session_id: String, path: String, include_hidden: bool) -> String {
    update_session(&session_id, "session_read_remote_dir", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "File access is disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_read_remote_dir(core_session_id, path.clone(), include_hidden);
        session.last_error = None;
        (true, format!("Requested remote directory {}", path))
    })
}

#[napi]
pub fn session_send_files(
    session_id: String,
    act_id: i32,
    path: String,
    to: String,
    file_num: i32,
    include_hidden: bool,
    is_remote: bool,
    is_dir: bool,
) -> String {
    update_session(&session_id, "session_send_files", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        if session.view_only {
            return (
                false,
                "File transfer is disabled for a view-only session".to_string(),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_send_files(
            core_session_id,
            act_id,
            path.clone(),
            to.clone(),
            file_num,
            include_hidden,
            is_remote,
            is_dir,
        );
        session.last_error = None;
        (true, format!("Queued transfer from {} to {}", path, to))
    })
}

#[napi]
pub fn session_set_confirm_override_file(
    session_id: String,
    act_id: i32,
    file_num: i32,
    need_override: bool,
    remember: bool,
    is_upload: bool,
) -> String {
    update_session(
        &session_id,
        "session_set_confirm_override_file",
        |session| {
            if session.phase == "closed" {
                return (false, "Session is closed".to_string());
            }
            if session.view_only {
                return (
                    false,
                    "File transfer is disabled for a view-only session".to_string(),
                );
            }
            let Some(core_session_id) = parse_core_session_id(session) else {
                return (false, "Missing core session id".to_string());
            };
            flutter_ffi::session_set_confirm_override_file(
                core_session_id,
                act_id,
                file_num,
                need_override,
                remember,
                is_upload,
            );
            session.last_error = None;
            (true, "Applied file overwrite decision".to_string())
        },
    )
}

#[napi]
pub fn session_cancel_job(session_id: String, act_id: i32) -> String {
    let core_session_id = {
        let mut sessions = session_store().lock().unwrap();
        let Some(session) = sessions.get_mut(&session_id) else {
            return action_response(
                "session_cancel_job",
                false,
                format!("Session {} was not found", session_id),
                None,
            );
        };
        session.last_action = "session_cancel_job".to_string();
        if session.phase == "closed" || session.phase == "closing" {
            return action_response(
                "session_cancel_job",
                false,
                "Session is closing or closed".to_string(),
                Some(session),
            );
        }
        if session.view_only {
            return action_response(
                "session_cancel_job",
                false,
                "File transfer is disabled for a view-only session".to_string(),
                Some(session),
            );
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return action_response(
                "session_cancel_job",
                false,
                "Missing core session id".to_string(),
                Some(session),
            );
        };
        core_session_id
    };

    flutter_ffi::session_cancel_job(core_session_id, act_id);

    let mut sessions = session_store().lock().unwrap();
    match sessions.get_mut(&session_id) {
        Some(session) => {
            session.last_error = None;
            action_response(
                "session_cancel_job",
                true,
                format!("Cancelled transfer job {}", act_id),
                Some(session),
            )
        }
        None => action_response(
            "session_cancel_job",
            true,
            format!("Cancelled transfer job {}", act_id),
            None,
        ),
    }
}

#[napi]
pub fn transfer_next_job_id() -> u32 {
    let next = flutter_ffi::main_get_common("transfer-job-id".to_string());
    next.parse::<u32>().unwrap_or(0)
}

// HarmonyOS controlled-device host surface. The state machine is deliberately
// owned by the HAR so repeated UI lifecycle calls are idempotent while Core owns
// the actual RustDesk host lifecycle and protocol state.
#[napi]
pub fn controlled_server_start(config_json: String) -> String {
    let _lifecycle = controlled_host_lifecycle().lock().unwrap();
    let mut config = match controlled_parse_json(
        "controlled_server_start",
        &config_json,
        MAX_CONTROLLED_JSON_BYTES,
    ) {
        Ok(value) if value.is_object() => value,
        Ok(_) => return json!({"ok":false,"action":"controlled_server_start","message":"config must be a JSON object"}).to_string(),
        Err(message) => return json!({"ok":false,"action":"controlled_server_start","message":message}).to_string(),
    };
    let audio_enabled = config
        .get("enableAudio")
        .or_else(|| config.get("audioEnabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !controlled_view_only_config_is_valid(&config) {
        return json!({
            "ok": false,
            "action": "controlled_server_start",
            "message": "HarmonyOS host currently supports watched/view-only sessions only"
        })
        .to_string();
    }
    config["enableInput"] = json!(false);
    config["enableClipboard"] = json!(false);
    config["requireLocalApproval"] = json!(true);
    let approve_mode = flutter_ffi::main_get_option("approve-mode".to_string());
    let approve_mode_fixed = flutter_ffi::main_is_option_fixed("approve-mode".to_string()).0;
    if approve_mode_fixed && approve_mode != "both" {
        return json!({
            "ok": false,
            "action": "controlled_server_start",
            "message": "watched/view-only hosting requires both password and local-click approval modes"
        })
        .to_string();
    }
    if !approve_mode_fixed && approve_mode != "both" {
        flutter_ffi::main_set_option("approve-mode".to_string(), "both".to_string());
    }
    let audio_fixed = controlled_option_fixed("enable-audio");
    if audio_fixed && !controlled_option_bool("enable-audio") {
        return json!({
            "ok": false,
            "action": "controlled_server_start",
            "message": "device audio sharing is disabled by policy"
        })
        .to_string();
    }
    if !audio_fixed {
        flutter_ffi::main_set_option("enable-audio".to_string(), "Y".to_string());
    }
    #[cfg(target_env = "ohos")]
    unsafe {
        OH_Input_CancelInjection();
        CONTROLLED_INPUT_AUTH_STATUS.store(-1, Ordering::Release);
    }
    CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON.store(INPUT_MOUSE_BUTTON_NONE, Ordering::Release);

    let host_was_started = ohos::host_is_started();
    let capture_was_active = controlled_native_capture_is_healthy();
    let screen_config = {
        let mut state = controlled_runtime().lock().unwrap();
        state.server_config = config;
        state.audio_enabled = audio_enabled;
        state.last_error = None;
        state.screen_config.clone()
    };
    if host_was_started && !capture_was_active {
        ohos::stop_host();
        let _ = controlled_screen_capture_stop();
    }
    if !capture_was_active
        && (CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) != 0
            || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0)
    {
        let _ = controlled_screen_capture_stop();
    }
    if !capture_was_active {
        {
            let mut state = controlled_runtime().lock().unwrap();
            state.running = false;
            state.audio_enabled = audio_enabled;
            state.last_error = None;
        }
        CONTROLLED_CAPTURE_START_PREPARING.store(true, Ordering::Release);
        let capture_response = controlled_screen_capture_start(screen_config.to_string());
        CONTROLLED_CAPTURE_START_PREPARING.store(false, Ordering::Release);
        let capture_ok = serde_json::from_str::<Value>(&capture_response)
            .ok()
            .and_then(|value| value.get("ok").and_then(Value::as_bool))
            .unwrap_or(false);
        if !capture_ok {
            ohos::stop_host();
            let mut state = controlled_runtime().lock().unwrap();
            state.running = false;
            state.audio_enabled = false;
            state.last_error = Some("screen and inner-audio capture failed to start".to_string());
            return controlled_response(
                "controlled_server_start",
                false,
                &state,
                json!({"message":"screen and inner-audio capture failed to start","captureResponse":capture_response}),
            );
        }
    }
    if !controlled_native_capture_is_healthy() {
        let _ = controlled_screen_capture_stop();
        ohos::stop_host();
        let mut state = controlled_runtime().lock().unwrap();
        state.running = false;
        state.audio_enabled = false;
        state.last_error = Some("capture became unavailable before host startup".to_string());
        return controlled_response(
            "controlled_server_start",
            false,
            &state,
            json!({"message":"capture became unavailable before host startup"}),
        );
    }
    if !ohos::start_host() || !ohos::host_is_started() || !controlled_native_capture_is_healthy() {
        let _ = controlled_screen_capture_stop();
        ohos::stop_host();
        let mut state = controlled_runtime().lock().unwrap();
        state.running = false;
        state.audio_enabled = false;
        state.last_error = Some("Core host failed to start".to_string());
        return controlled_response(
            "controlled_server_start",
            false,
            &state,
            json!({"message":"Core host failed to start"}),
        );
    }
    let mut state = controlled_runtime().lock().unwrap();
    state.running = true;
    if !host_was_started || !capture_was_active {
        state.generation = state.generation.saturating_add(1);
    }
    controlled_response(
        "controlled_server_start",
        true,
        &state,
        json!({"idempotent":host_was_started && capture_was_active,"coreHostBridgeAvailable":true,"audioEnabled":state.audio_enabled,"captureActive":true,"manualApprovalRequired":true}),
    )
}

#[napi]
pub fn controlled_server_stop() -> String {
    let _lifecycle = controlled_host_lifecycle().lock().unwrap();
    {
        let mut state = controlled_runtime().lock().unwrap();
        state.running = false;
        state.generation = state.generation.saturating_add(1);
    }
    let mut capture_message: Option<String> = None;
    if CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) != 0
        || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0
    {
        let response = controlled_screen_capture_stop();
        if let Ok(value) = serde_json::from_str::<Value>(&response) {
            if value.get("ok").and_then(Value::as_bool) != Some(true) {
                capture_message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .or_else(|| Some("screen or inner-audio capture failed to stop".to_string()));
            }
        }
    }
    ohos::stop_host();
    CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON.store(INPUT_MOUSE_BUTTON_NONE, Ordering::Release);
    let mut state = controlled_runtime().lock().unwrap();
    state.running = false;
    state.audio_enabled = false;
    state.incoming.clear();
    state.input.clear();
    state.clipboard.clear();
    let capture_released = CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) == 0
        && CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) == 0;
    if !capture_released && capture_message.is_none() {
        capture_message = Some("native capture resources remain allocated".to_string());
    }
    if let Some(message) = capture_message.clone() {
        state.last_error = Some(message);
    }
    controlled_response(
        "controlled_server_stop",
        capture_released && capture_message.is_none(),
        &state,
        json!({"idempotent":true,"captureReleased":capture_released,"message":capture_message}),
    )
}

#[napi]
pub fn controlled_server_get_status() -> String {
    let mut state = controlled_runtime().lock().unwrap();
    state.running = ohos::host_is_started();
    let audio_policy_enabled = controlled_option_bool("enable-audio");
    if state.running && !audio_policy_enabled {
        state.last_error = Some("device audio sharing was disabled by policy".to_string());
    }
    let status_healthy = state.running && audio_policy_enabled;
    let (screen_width, screen_height) = ohos::host_screen_size();
    let (clients, _) = controlled_clients_payload(state.generation);
    controlled_response(
        "controlled_server_get_status",
        status_healthy,
        &state,
        json!({
          "capabilities": state.capabilities,
          "audioEnabled": state.audio_enabled,
          "screenFramesPushed": state.pushed_screen_frames,
          "audioFramesPushed": state.pushed_audio_frames,
          "queueDepths": {"incoming":state.incoming.len(),"input":state.input.len(),"clipboard":state.clipboard.len()},
          "coreHostBridgeAvailable":true,"clientCount":ohos::host_client_count(),"clients":clients,"manualApprovalRequired":true,
          "state":if state.running { "ready" } else { "disabled" },"serverRunning":state.running,
          "myId":flutter_ffi::main_get_my_id(),"temporaryPassword":flutter_ffi::main_get_temporary_password(),
          "screenSize":{"width":screen_width,"height":screen_height}
        }),
    )
}

#[napi]
pub fn controlled_incoming_poll(limit: u32) -> String {
    let state = controlled_runtime().lock().unwrap();
    let (mut clients, mut requests) = controlled_clients_payload(state.generation);
    clients.truncate(limit.min(MAX_CONTROLLED_QUEUE_ITEMS as u32) as usize);
    requests.truncate(limit.min(MAX_CONTROLLED_QUEUE_ITEMS as u32) as usize);
    controlled_response(
        "controlled_incoming_poll",
        true,
        &state,
        json!({"clients":clients,"requests":requests,"clientCount":ohos::host_client_count(),"manualApprovalRequired":true}),
    )
}

#[napi]
pub fn controlled_incoming_resolve(request_id: String, accepted: bool) -> String {
    let _lifecycle = controlled_host_lifecycle().lock().unwrap();
    if request_id.trim().is_empty() || request_id.len() > 256 {
        return json!({"ok":false,"action":"controlled_incoming_resolve","message":"requestId must contain 1..256 bytes"}).to_string();
    }
    let Some((generation_text, id_text)) = request_id.trim().split_once(':') else {
        return json!({"ok":false,"action":"controlled_incoming_resolve","message":"requestId must contain host generation and Core connection id"}).to_string();
    };
    let (Ok(generation), Ok(id)) = (generation_text.parse::<u64>(), id_text.parse::<i32>()) else {
        return json!({"ok":false,"action":"controlled_incoming_resolve","message":"requestId contains an invalid host generation or Core connection id"}).to_string();
    };
    let state = controlled_runtime().lock().unwrap();
    if !state.running || state.generation != generation || !ohos::host_is_started() {
        return json!({
            "ok": false,
            "action": "controlled_incoming_resolve",
            "message": "request belongs to a stale or stopped host generation"
        })
        .to_string();
    }
    let forwarded = if accepted {
        ohos::host_authorize_client(id)
    } else {
        ohos::host_close_client(id)
    };
    if !forwarded {
        return json!({
            "ok": false,
            "action": "controlled_incoming_resolve",
            "message": "request is stale or its Core connection is no longer available"
        })
        .to_string();
    }
    controlled_response(
        "controlled_incoming_resolve",
        state.running,
        &state,
        json!({"requestId":request_id,"accepted":accepted,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_incoming_set_permission(
    request_id: String,
    permission: String,
    enabled: bool,
) -> String {
    let allowed = [
        "keyboard",
        "clipboard",
        "audio",
        "file",
        "restart",
        "recording",
    ];
    if request_id.trim().is_empty() || !allowed.contains(&permission.as_str()) {
        return json!({"ok":false,"action":"controlled_incoming_set_permission","message":"invalid requestId or permission"}).to_string();
    }
    if !controlled_view_only_permission_is_valid(&permission, enabled) {
        return json!({
            "ok": false,
            "action": "controlled_incoming_set_permission",
            "message": "only screen and audio are available in watched/view-only mode"
        })
        .to_string();
    }
    let Ok(id) = request_id.trim().parse::<i32>() else {
        return json!({"ok":false,"action":"controlled_incoming_set_permission","message":"requestId must be a numeric Core connection id"}).to_string();
    };
    flutter_ffi::cm_switch_permission(id, permission.clone(), enabled);
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_incoming_set_permission",
        state.running,
        &state,
        json!({"requestId":request_id,"permission":permission,"enabled":enabled,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_password_get() -> String {
    let verification_method = flutter_ffi::main_get_option("verification-method".to_string());
    let permanent_password_set =
        flutter_ffi::main_get_common("permanent-password-set".to_string()) == "true";
    let local_permanent_password_set =
        flutter_ffi::main_get_common("local-permanent-password-set".to_string()) == "true";
    let verification_method_fixed =
        flutter_ffi::main_is_option_fixed("verification-method".to_string()).0;
    let permanent_password_change_disabled =
        flutter_ffi::main_get_buildin_option("disable-change-permanent-password".to_string()).0
            == "Y";
    let max_password_length = flutter_ffi::main_max_encrypt_len().0;
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_password_get",
        true,
        &state,
        json!({
            "temporaryPassword":flutter_ffi::main_get_temporary_password(),
            "verificationMethod":verification_method,
            "permanentPasswordSet":permanent_password_set,
            "localPermanentPasswordSet":local_permanent_password_set,
            "verificationMethodFixed":verification_method_fixed,
            "permanentPasswordChangeDisabled":permanent_password_change_disabled,
            "maxPasswordLength":max_password_length,
            "forwardedToCore":true
        }),
    )
}

#[napi]
pub fn controlled_password_set_verification_method(method: String) -> String {
    const ALLOWED: [&str; 3] = [
        "use-temporary-password",
        "use-permanent-password",
        "use-both-passwords",
    ];
    if !ALLOWED.contains(&method.as_str()) {
        return json!({"ok":false,"action":"controlled_password_set_verification_method","message":"invalid verification method"}).to_string();
    }
    if flutter_ffi::main_is_option_fixed("verification-method".to_string()).0 {
        return json!({"ok":false,"action":"controlled_password_set_verification_method","message":"verification method is fixed by policy"}).to_string();
    }
    if method == "use-permanent-password"
        && flutter_ffi::main_get_common("permanent-password-set".to_string()) != "true"
    {
        return json!({"ok":false,"action":"controlled_password_set_verification_method","message":"a permanent password must be configured first"}).to_string();
    }
    flutter_ffi::main_set_option("verification-method".to_string(), method.clone());
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_password_set_verification_method",
        true,
        &state,
        json!({"verificationMethod":method,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_password_refresh() -> String {
    flutter_ffi::main_update_temporary_password();
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_password_refresh",
        true,
        &state,
        json!({"temporaryPassword":flutter_ffi::main_get_temporary_password(),"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_password_set_permanent(password: String) -> String {
    if password.len() > MAX_CONTROLLED_PASSWORD_BYTES || password.chars().any(char::is_control) {
        return json!({"ok":false,"action":"controlled_password_set_permanent","message":"password exceeds bounds or contains control characters"}).to_string();
    }
    let configured = !password.is_empty();
    let applied = flutter_ffi::main_set_permanent_password_with_result(password);
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_password_set_permanent",
        applied,
        &state,
        json!({"configured":configured && applied,"forwardedToCore":true}),
    )
}

fn controlled_option_bool(key: &str) -> bool {
    let value = flutter_ffi::main_get_option(key.to_string());
    hbb_common::config::option2bool(key, &value)
}

fn controlled_option_fixed(key: &str) -> bool {
    flutter_ffi::main_is_option_fixed(key.to_string()).0
}

#[napi]
pub fn controlled_settings_get() -> String {
    let temporary_password_length =
        flutter_ffi::main_get_option("temporary-password-length".to_string());
    let temporary_password_length = match temporary_password_length.as_str() {
        "8" => "8",
        "10" => "10",
        _ => "6",
    };
    let direct_access_port = flutter_ffi::main_get_option("direct-access-port".to_string());
    let auto_disconnect_timeout =
        flutter_ffi::main_get_option("auto-disconnect-timeout".to_string());
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_settings_get",
        true,
        &state,
        json!({
            "approveMode": "both",
            "temporaryPasswordLength": temporary_password_length,
            "numericOneTimePassword": controlled_option_bool("allow-numeric-one-time-password"),
            "enableKeyboard": false,
            "enableClipboard": false,
            "enableAudio": true,
            "enableLanDiscovery": controlled_option_bool("enable-lan-discovery"),
            "ipWhitelist": flutter_ffi::main_get_option("whitelist".to_string()),
            "idWhitelist": flutter_ffi::main_get_option("id-whitelist".to_string()),
            "directIpAccess": controlled_option_bool("direct-server"),
            "directAccessPort": if direct_access_port.is_empty() { "21118" } else { direct_access_port.as_str() },
            "autoDisconnect": controlled_option_bool("allow-auto-disconnect"),
            "autoDisconnectTimeout": if auto_disconnect_timeout.is_empty() { "10" } else { auto_disconnect_timeout.as_str() },
            "keepAwakeDuringIncomingSessions": controlled_option_bool("keep-awake-during-incoming-sessions"),
            "allowRemoteConfigModification": controlled_option_bool("allow-remote-config-modification"),
            "trustedDevicesEnabled": controlled_option_bool("enable-trusted-devices"),
            "fixed": {
                "approveMode": true,
                "temporaryPasswordLength": controlled_option_fixed("temporary-password-length"),
                "numericOneTimePassword": controlled_option_fixed("allow-numeric-one-time-password"),
                "enableKeyboard": true,
                "enableClipboard": true,
                "enableAudio": true,
                "enableLanDiscovery": controlled_option_fixed("enable-lan-discovery"),
                "ipWhitelist": controlled_option_fixed("whitelist"),
                "idWhitelist": controlled_option_fixed("id-whitelist"),
                "directIpAccess": controlled_option_fixed("direct-server"),
                "directAccessPort": controlled_option_fixed("direct-access-port"),
                "autoDisconnect": controlled_option_fixed("allow-auto-disconnect"),
                "autoDisconnectTimeout": controlled_option_fixed("auto-disconnect-timeout"),
                "keepAwakeDuringIncomingSessions": controlled_option_fixed("keep-awake-during-incoming-sessions"),
                "allowRemoteConfigModification": controlled_option_fixed("allow-remote-config-modification"),
                "trustedDevicesEnabled": controlled_option_fixed("enable-trusted-devices")
            },
            "forwardedToCore": true
        }),
    )
}

#[napi]
pub fn controlled_setting_set(key: String, value: String) -> String {
    if key.len() > 64 || value.len() > MAX_CONTROLLED_JSON_BYTES {
        return json!({"ok":false,"action":"controlled_setting_set","message":"setting exceeds bounds"}).to_string();
    }
    if key == "enableKeyboard" || key == "enableClipboard" || key == "enableAudio" {
        return json!({
            "ok": false,
            "action": "controlled_setting_set",
            "message": "screen and audio are fixed on while input and clipboard are fixed off"
        })
        .to_string();
    }
    if key == "approveMode" {
        return json!({
            "ok": false,
            "action": "controlled_setting_set",
            "message": "watched/view-only hosting always offers local approval or password authentication"
        })
        .to_string();
    }
    let option = match key.as_str() {
        "temporaryPasswordLength" => "temporary-password-length",
        "numericOneTimePassword" => "allow-numeric-one-time-password",
        "enableLanDiscovery" => "enable-lan-discovery",
        "ipWhitelist" => "whitelist",
        "idWhitelist" => "id-whitelist",
        "directIpAccess" => "direct-server",
        "directAccessPort" => "direct-access-port",
        "autoDisconnect" => "allow-auto-disconnect",
        "autoDisconnectTimeout" => "auto-disconnect-timeout",
        "keepAwakeDuringIncomingSessions" => "keep-awake-during-incoming-sessions",
        "allowRemoteConfigModification" => "allow-remote-config-modification",
        "trustedDevicesEnabled" => "enable-trusted-devices",
        _ => return json!({"ok":false,"action":"controlled_setting_set","message":"unsupported setting"}).to_string(),
    };
    if controlled_option_fixed(option) {
        return json!({"ok":false,"action":"controlled_setting_set","message":"setting is fixed by policy"}).to_string();
    }
    let stored = match key.as_str() {
        "temporaryPasswordLength" => match value.as_str() {
            "6" | "8" | "10" => value.clone(),
            _ => return json!({"ok":false,"action":"controlled_setting_set","message":"invalid temporary password length"}).to_string(),
        },
        "directAccessPort" => {
            let Ok(port) = value.parse::<u16>() else {
                return json!({"ok":false,"action":"controlled_setting_set","message":"invalid direct access port"}).to_string();
            };
            if port == 0 {
                return json!({"ok":false,"action":"controlled_setting_set","message":"invalid direct access port"}).to_string();
            }
            port.to_string()
        }
        "autoDisconnectTimeout" => {
            let Ok(minutes) = value.parse::<u16>() else {
                return json!({"ok":false,"action":"controlled_setting_set","message":"invalid auto disconnect timeout"}).to_string();
            };
            minutes.to_string()
        }
        "ipWhitelist" | "idWhitelist" => value.trim().to_string(),
        _ => match value.as_str() {
            "true" => "Y".to_string(),
            "false" => "N".to_string(),
            _ => return json!({"ok":false,"action":"controlled_setting_set","message":"boolean setting must be true or false"}).to_string(),
        },
    };
    flutter_ffi::main_set_option(option.to_string(), stored);
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_setting_set",
        true,
        &state,
        json!({"key":key,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_two_factor_get() -> String {
    let trusted_devices = serde_json::from_str::<Value>(&flutter_ffi::main_get_trusted_devices())
        .unwrap_or_else(|_| json!([]));
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_get",
        true,
        &state,
        json!({
            "enabled": flutter_ffi::main_has_valid_2fa_sync().0,
            "trustedDevicesEnabled": controlled_option_bool("enable-trusted-devices"),
            "trustedDevicesFixed": controlled_option_fixed("enable-trusted-devices"),
            "trustedDevices": trusted_devices,
            "forwardedToCore": true
        }),
    )
}

#[napi]
pub fn controlled_two_factor_begin() -> String {
    if flutter_ffi::main_has_valid_2fa_sync().0 {
        return json!({"ok":false,"action":"controlled_two_factor_begin","message":"2FA is already enabled"}).to_string();
    }
    let uri = flutter_ffi::main_generate2fa();
    if uri.is_empty() {
        return json!({"ok":false,"action":"controlled_two_factor_begin","message":"Core failed to generate 2FA secret"}).to_string();
    }
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_begin",
        true,
        &state,
        json!({"uri":uri,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_two_factor_verify(code: String) -> String {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return json!({"ok":false,"action":"controlled_two_factor_verify","message":"verification code must contain six digits"}).to_string();
    }
    let verified = flutter_ffi::main_verify2fa(code);
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_verify",
        verified,
        &state,
        json!({"enabled":flutter_ffi::main_has_valid_2fa_sync().0,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_two_factor_disable() -> String {
    flutter_ffi::main_disable2fa();
    let enabled = flutter_ffi::main_has_valid_2fa_sync().0;
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_disable",
        !enabled,
        &state,
        json!({"enabled":enabled,"trustedDevicesCleared":true,"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_two_factor_remove_trusted_devices(hwids_json: String) -> String {
    let hwids = match controlled_parse_json(
        "controlled_two_factor_remove_trusted_devices",
        &hwids_json,
        MAX_CONTROLLED_JSON_BYTES,
    ) {
        Ok(value) if value.is_array() => value,
        Ok(_) => return json!({"ok":false,"action":"controlled_two_factor_remove_trusted_devices","message":"hwids must be a JSON array"}).to_string(),
        Err(message) => return json!({"ok":false,"action":"controlled_two_factor_remove_trusted_devices","message":message}).to_string(),
    };
    flutter_ffi::main_remove_trusted_devices(hwids.to_string());
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_remove_trusted_devices",
        true,
        &state,
        json!({"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_two_factor_clear_trusted_devices() -> String {
    flutter_ffi::main_clear_trusted_devices();
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_two_factor_clear_trusted_devices",
        true,
        &state,
        json!({"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_screen_configure(config_json: String) -> String {
    let config = match controlled_parse_json("controlled_screen_configure", &config_json, MAX_CONTROLLED_JSON_BYTES) { Ok(v) if v.is_object() => v, Ok(_) => return json!({"ok":false,"action":"controlled_screen_configure","message":"config must be a JSON object"}).to_string(), Err(m) => return json!({"ok":false,"action":"controlled_screen_configure","message":m}).to_string() };
    let width = config.get("width").and_then(Value::as_u64).unwrap_or(0);
    let height = config.get("height").and_then(Value::as_u64).unwrap_or(0);
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return json!({"ok":false,"action":"controlled_screen_configure","message":"width and height must be within 1..16384"}).to_string();
    }
    // Keep the RGBA stream bounded so desktop decoders receive frames promptly
    // while retaining the source aspect ratio.
    let (stream_width, stream_height) =
        controlled_pixelmap_stream_size(width as usize, height as usize);
    let previous = ohos::host_screen_size();
    let forwarded = ohos::configure_host_screen(stream_width, stream_height);
    let current = ohos::host_screen_size();
    let changed = forwarded && previous != current;
    let mut state = controlled_runtime().lock().unwrap();
    if forwarded {
        let mut normalized_config = config;
        normalized_config["sourceWidth"] = json!(width);
        normalized_config["sourceHeight"] = json!(height);
        normalized_config["width"] = json!(stream_width);
        normalized_config["height"] = json!(stream_height);
        state.screen_config = normalized_config;
    } else {
        state.last_error = Some("Core rejected controlled-host screen geometry".to_string());
    }
    controlled_response(
        "controlled_screen_configure",
        forwarded,
        &state,
        json!({"nativeCaptureStartStopAvailable":cfg!(target_env = "ohos"),"changed":changed,"forwardedToCore":forwarded,"width":current.0,"height":current.1,"sourceWidth":width,"sourceHeight":height}),
    )
}

#[napi]
pub fn controlled_screen_push_frame(
    frame: napi_ohos::bindgen_prelude::Uint8Array,
    metadata_json: String,
) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_screen_push_frame");
    }
    if frame.len() > MAX_CONTROLLED_FRAME_BYTES {
        return json!({"ok":false,"action":"controlled_screen_push_frame","message":"frame exceeds 32 MiB"}).to_string();
    }
    let metadata = match controlled_parse_json(
        "controlled_screen_push_frame",
        &metadata_json,
        MAX_CONTROLLED_JSON_BYTES,
    ) {
        Ok(value) => value,
        Err(message) => {
            return json!({"ok":false,"action":"controlled_screen_push_frame","message":message})
                .to_string()
        }
    };
    let width = metadata.get("width").and_then(Value::as_u64).unwrap_or(0) as usize;
    let height = metadata.get("height").and_then(Value::as_u64).unwrap_or(0) as usize;
    if width == 0 || height == 0 || frame.len() < width.saturating_mul(height).saturating_mul(4) {
        return json!({"ok":false,"action":"controlled_screen_push_frame","message":"metadata width/height or RGBA byte length is invalid"}).to_string();
    }
    let mut state = controlled_runtime().lock().unwrap();
    if !state.running {
        return controlled_response(
            "controlled_screen_push_frame",
            false,
            &state,
            json!({"message":"server is stopped"}),
        );
    }
    let forwarded = ohos::push_host_screen_frame_rgba(frame.as_ref(), width, height);
    if forwarded {
        state.pushed_screen_frames = state.pushed_screen_frames.saturating_add(1);
    }
    controlled_response(
        "controlled_screen_push_frame",
        forwarded,
        &state,
        json!({"bytes":frame.len(),"width":width,"height":height,"forwardedToCore":forwarded}),
    )
}

#[napi]
pub fn controlled_input_poll(limit: u32) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_input_poll");
    }
    let state = controlled_runtime().lock().unwrap();
    let display_id = state
        .screen_config
        .get("displayId")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stream_size = (
        state
            .screen_config
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        state
            .screen_config
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    );
    let source_size = (
        state
            .screen_config
            .get("sourceWidth")
            .and_then(Value::as_u64)
            .unwrap_or(stream_size.0),
        state
            .screen_config
            .get("sourceHeight")
            .and_then(Value::as_u64)
            .unwrap_or(stream_size.1),
    );
    let mut events = Vec::new();
    for _ in 0..limit.min(MAX_CONTROLLED_QUEUE_ITEMS as u32) {
        let Some(event_json) = ohos::poll_host_input_event_json() else {
            break;
        };
        let Ok(event) = serde_json::from_str::<Value>(&event_json) else {
            continue;
        };
        let sequence = CONTROLLED_INPUT_SEQUENCE
            .fetch_add(1, Ordering::Relaxed)
            .to_string();
        events.extend(controlled_input_events_from_core(
            &event,
            sequence,
            display_id,
            stream_size,
            source_size,
        ));
    }
    controlled_response(
        "controlled_input_poll",
        true,
        &state,
        json!({"events":events,"forwardedFromCore":true}),
    )
}

#[napi]
pub fn controlled_input_ack(event_id: String, success: bool) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_input_ack");
    }
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_input_ack",
        state.running && !event_id.trim().is_empty() && event_id.len() <= 256,
        &state,
        json!({"eventId":event_id,"success":success,"ackRequiredByCore":false}),
    )
}

fn controlled_native_mouse_button(button: i64) -> Option<i32> {
    match button {
        0 => Some(0),
        1 => Some(1),
        2 => Some(2),
        // InputKit uses 5/6 for forward/back, while the native input API uses 3/4.
        5 => Some(3),
        6 => Some(4),
        _ => None,
    }
}

fn controlled_wheel_touch_points(axis: i32, x: i32, y: i32, value: f32) -> [(i32, i32); 5] {
    // API 26 currently accepts simulated mouse-axis events but routes them
    // with an unusable synthetic axis id, so ArkUI/Web components receive no
    // displacement. Model one discrete wheel step as a short touch pan at the
    // cursor instead; both native Scroll and Web components consume this path.
    let distance = (value.abs().clamp(1.0, 4.0) * 96.0).round() as i32;
    let direction = if value > 0.0 { -1 } else { 1 };
    let start_offset = -(direction * (distance / 2));
    let end_offset = direction * (distance / 2);
    let offsets = [
        start_offset,
        start_offset / 2,
        0,
        end_offset / 2,
        end_offset,
    ];
    offsets.map(|offset| {
        if axis == 0 {
            (x.max(0), y.saturating_add(offset).max(0))
        } else {
            (x.saturating_add(offset).max(0), y.max(0))
        }
    })
}

#[cfg(target_env = "ohos")]
unsafe fn controlled_mouse_action_time() -> i64 {
    const CLOCK_MONOTONIC: i32 = 1;
    let mut time = OhosTimespec::default();
    // InputKit's PointerEvent actionTime uses monotonic microseconds, matching
    // multimodalinput_input::GetSysClockTime(). Milliseconds are treated as
    // stale events after they reach the input service.
    let now = if clock_gettime(CLOCK_MONOTONIC, &mut time) == 0 {
        time.tv_sec
            .saturating_mul(1_000_000)
            .saturating_add(time.tv_nsec / 1_000)
    } else {
        0
    };
    let mut previous = CONTROLLED_GLOBAL_MOUSE_ACTION_TIME.load(Ordering::Acquire);
    loop {
        let next = now.max(previous.saturating_add(1));
        match CONTROLLED_GLOBAL_MOUSE_ACTION_TIME.compare_exchange_weak(
            previous,
            next,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return next,
            Err(actual) => previous = actual,
        }
    }
}

#[cfg(target_env = "ohos")]
unsafe fn controlled_prepare_global_mouse_event(
    mouse_event: *mut InputMouseEvent,
    display_id: i32,
    x: i32,
    y: i32,
    button: i32,
) {
    OH_Input_SetMouseEventDisplayId(mouse_event, display_id);
    OH_Input_SetMouseEventDisplayX(mouse_event, x);
    OH_Input_SetMouseEventDisplayY(mouse_event, y);
    OH_Input_SetMouseEventGlobalX(mouse_event, x);
    OH_Input_SetMouseEventGlobalY(mouse_event, y);
    OH_Input_SetMouseEventButton(mouse_event, button);
    OH_Input_SetMouseEventActionTime(mouse_event, controlled_mouse_action_time());
}

#[napi]
pub fn controlled_input_inject_mouse_global(event_json: String) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_input_inject_mouse_global");
    }
    let event = match controlled_parse_json(
        "controlled_input_inject_mouse_global",
        &event_json,
        MAX_CONTROLLED_JSON_BYTES,
    ) {
        Ok(value) if value.is_object() => value,
        Ok(_) => {
            return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"event must be a JSON object"}).to_string()
        }
        Err(message) => {
            return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":message}).to_string()
        }
    };
    if event.get("type").and_then(Value::as_str) != Some("mouse") {
        return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"only mouse events are supported"}).to_string();
    }

    #[cfg(target_env = "ohos")]
    unsafe {
        let action = event
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let display_id = event
            .get("displayId")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(0);
        let supplied_x = event
            .get("x")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok());
        let supplied_y = event
            .get("y")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok());
        if let Some(x) = supplied_x {
            CONTROLLED_GLOBAL_MOUSE_X.store(x, Ordering::Release);
        }
        if let Some(y) = supplied_y {
            CONTROLLED_GLOBAL_MOUSE_Y.store(y, Ordering::Release);
        }
        let x = supplied_x.unwrap_or_else(|| CONTROLLED_GLOBAL_MOUSE_X.load(Ordering::Acquire));
        let y = supplied_y.unwrap_or_else(|| CONTROLLED_GLOBAL_MOUSE_Y.load(Ordering::Acquire));

        let mut mouse_event = OH_Input_CreateMouseEvent();
        if mouse_event.is_null() {
            return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"failed to create native mouse event"}).to_string();
        }

        let result = match action {
            "move" => {
                let pressed_button = CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON.load(Ordering::Acquire);
                controlled_prepare_global_mouse_event(
                    mouse_event,
                    display_id,
                    x,
                    y,
                    pressed_button,
                );
                OH_Input_SetMouseEventAction(mouse_event, INPUT_MOUSE_ACTION_MOVE);
                vec![OH_Input_InjectMouseEventGlobal(mouse_event)]
            }
            "button_down" | "down" | "button_up" | "up" => {
                let Some(button) = event
                    .get("button")
                    .and_then(Value::as_i64)
                    .and_then(controlled_native_mouse_button)
                else {
                    OH_Input_DestroyMouseEvent(&mut mouse_event);
                    return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"unsupported mouse button"}).to_string();
                };
                controlled_prepare_global_mouse_event(mouse_event, display_id, x, y, button);
                OH_Input_SetMouseEventAction(
                    mouse_event,
                    if action == "button_down" || action == "down" {
                        INPUT_MOUSE_ACTION_BUTTON_DOWN
                    } else {
                        INPUT_MOUSE_ACTION_BUTTON_UP
                    },
                );
                let code = OH_Input_InjectMouseEventGlobal(mouse_event);
                if code == INPUT_SUCCESS {
                    if action == "button_down" || action == "down" {
                        CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON.store(button, Ordering::Release);
                    } else {
                        CONTROLLED_GLOBAL_MOUSE_PRESSED_BUTTON
                            .store(INPUT_MOUSE_BUTTON_NONE, Ordering::Release);
                    }
                }
                vec![code]
            }
            "axis" | "wheel" | "trackpad" => {
                let axis = event
                    .get("axis")
                    .and_then(Value::as_i64)
                    .and_then(|value| i32::try_from(value).ok())
                    .unwrap_or(0);
                if axis != 0 && axis != 1 {
                    OH_Input_DestroyMouseEvent(&mut mouse_event);
                    return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"unsupported mouse axis"}).to_string();
                }
                let value = event
                    .get("value")
                    .and_then(Value::as_f64)
                    .unwrap_or_default() as f32;
                if value == 0.0 {
                    OH_Input_DestroyMouseEvent(&mut mouse_event);
                    return json!({"ok":true,"action":"controlled_input_inject_mouse_global","nativeCodes":[],"ignoredZeroAxis":true}).to_string();
                }
                let mut touch_event = OH_Input_CreateTouchEvent();
                if touch_event.is_null() {
                    OH_Input_DestroyMouseEvent(&mut mouse_event);
                    return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"failed to create native touch event for wheel"}).to_string();
                }
                OH_Input_SetTouchEventFingerId(touch_event, 0);
                OH_Input_SetTouchEventDisplayId(touch_event, display_id);
                let points = controlled_wheel_touch_points(axis, x, y, value);
                let mut codes = Vec::with_capacity(points.len());
                for (index, (touch_x, touch_y)) in points.into_iter().enumerate() {
                    if index > 0 {
                        std::thread::sleep(Duration::from_millis(16));
                    }
                    OH_Input_SetTouchEventDisplayX(touch_event, touch_x);
                    OH_Input_SetTouchEventDisplayY(touch_event, touch_y);
                    OH_Input_SetTouchEventGlobalX(touch_event, touch_x);
                    OH_Input_SetTouchEventGlobalY(touch_event, touch_y);
                    OH_Input_SetTouchEventActionTime(touch_event, controlled_mouse_action_time());
                    OH_Input_SetTouchEventAction(
                        touch_event,
                        if index == 0 {
                            INPUT_TOUCH_ACTION_DOWN
                        } else if index + 1 == points.len() {
                            INPUT_TOUCH_ACTION_UP
                        } else {
                            INPUT_TOUCH_ACTION_MOVE
                        },
                    );
                    codes.push(OH_Input_InjectTouchEventGlobal(touch_event));
                }
                OH_Input_DestroyTouchEvent(&mut touch_event);
                codes
            }
            _ => {
                OH_Input_DestroyMouseEvent(&mut mouse_event);
                return json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"unsupported mouse action"}).to_string();
            }
        };
        OH_Input_DestroyMouseEvent(&mut mouse_event);
        let ok = result.iter().all(|code| *code == 0);
        return json!({
          "ok":ok,
          "action":"controlled_input_inject_mouse_global",
          "nativeCodes":result,
          "globalX":x,
          "globalY":y,
          "globalInjection":true
        })
        .to_string();
    }

    #[cfg(not(target_env = "ohos"))]
    json!({"ok":false,"action":"controlled_input_inject_mouse_global","message":"global mouse injection is only available on OHOS"}).to_string()
}

#[napi]
pub fn controlled_clipboard_push(content_json: String) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_clipboard_push");
    }
    let content = match controlled_parse_json(
        "controlled_clipboard_push",
        &content_json,
        MAX_CONTROLLED_CLIPBOARD_BYTES,
    ) {
        Ok(v) => v,
        Err(m) => {
            return json!({"ok":false,"action":"controlled_clipboard_push","message":m}).to_string()
        }
    };
    let Some(text) = content.get("text").and_then(Value::as_str) else {
        return json!({"ok":false,"action":"controlled_clipboard_push","message":"content.text must be a string"}).to_string();
    };
    let clipboards = MultiClipboards {
        clipboards: vec![Clipboard {
            content: text.as_bytes().to_vec().into(),
            format: ClipboardFormat::Text.into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    ohos::update_clipboards(false, clipboards);
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_clipboard_push",
        state.running,
        &state,
        json!({"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_clipboard_poll(limit: u32) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_clipboard_poll");
    }
    let state = controlled_runtime().lock().unwrap();
    let mut events = Vec::new();
    if limit > 0 {
        if let Some(clipboards) = ohos::take_host_received_clipboards() {
            for clipboard in clipboards
                .clipboards
                .into_iter()
                .take(limit.min(MAX_CONTROLLED_QUEUE_ITEMS as u32) as usize)
            {
                events.push(json!({"format":clipboard.format.value(),"text":String::from_utf8_lossy(&clipboard.content).to_string(),"bytes":clipboard.content.len()}));
            }
        }
    }
    controlled_response(
        "controlled_clipboard_poll",
        true,
        &state,
        json!({"events":events,"forwardedFromCore":true}),
    )
}

#[napi]
pub fn controlled_audio_configure(config_json: String) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_audio_configure");
    }
    let config = match controlled_parse_json("controlled_audio_configure", &config_json, MAX_CONTROLLED_JSON_BYTES) { Ok(v) if v.is_object() => v, Ok(_) => return json!({"ok":false,"action":"controlled_audio_configure","message":"config must be a JSON object"}).to_string(), Err(m) => return json!({"ok":false,"action":"controlled_audio_configure","message":m}).to_string() };
    let mut state = controlled_runtime().lock().unwrap();
    state.audio_config = config;
    controlled_response("controlled_audio_configure", true, &state, json!({}))
}

#[napi]
pub fn controlled_audio_push_frame(
    frame: napi_ohos::bindgen_prelude::Uint8Array,
    metadata_json: String,
) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_audio_push_frame");
    }
    if frame.len() > MAX_CONTROLLED_FRAME_BYTES {
        return json!({"ok":false,"action":"controlled_audio_push_frame","message":"frame exceeds 32 MiB"}).to_string();
    }
    if let Err(m) = controlled_parse_json(
        "controlled_audio_push_frame",
        &metadata_json,
        MAX_CONTROLLED_JSON_BYTES,
    ) {
        return json!({"ok":false,"action":"controlled_audio_push_frame","message":m}).to_string();
    }
    let mut state = controlled_runtime().lock().unwrap();
    if !state.running || !state.audio_enabled {
        return controlled_response(
            "controlled_audio_push_frame",
            false,
            &state,
            json!({"message":"server or audio is disabled"}),
        );
    }
    ohos::push_host_audio_f32_stereo(frame.as_ref());
    state.pushed_audio_frames = state.pushed_audio_frames.saturating_add(1);
    controlled_response(
        "controlled_audio_push_frame",
        true,
        &state,
        json!({"bytes":frame.len(),"forwardedToCore":true}),
    )
}

#[napi]
pub fn controlled_audio_set_enabled(enabled: bool) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_audio_set_enabled");
    }
    let mut state = controlled_runtime().lock().unwrap();
    state.audio_enabled = enabled && state.running;
    controlled_response(
        "controlled_audio_set_enabled",
        enabled == state.audio_enabled,
        &state,
        json!({"enabled":state.audio_enabled,"coreConsumesFramesWhenEnabled":true}),
    )
}

#[napi]
pub fn controlled_device_set_capabilities(capabilities_json: String) -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_device_set_capabilities");
    }
    let capabilities = match controlled_parse_json("controlled_device_set_capabilities", &capabilities_json, MAX_CONTROLLED_JSON_BYTES) { Ok(v) if v.is_object() => v, Ok(_) => return json!({"ok":false,"action":"controlled_device_set_capabilities","message":"capabilities must be a JSON object"}).to_string(), Err(m) => return json!({"ok":false,"action":"controlled_device_set_capabilities","message":m}).to_string() };
    let mut state = controlled_runtime().lock().unwrap();
    state.capabilities = capabilities;
    controlled_response(
        "controlled_device_set_capabilities",
        true,
        &state,
        json!({"forwardedToCore":false}),
    )
}

#[napi]
pub fn controlled_device_get_capabilities() -> String {
    let state = controlled_runtime().lock().unwrap();
    controlled_response(
        "controlled_device_get_capabilities",
        true,
        &state,
        json!({"capabilities":state.capabilities,"nativeScreenCapture":{"available":cfg!(target_env = "ohos"),"framesForwardedToCore":true},"inputDialogAuthorization":{"available":false,"independentOfControlDevice":false}}),
    )
}

#[napi]
pub fn controlled_screen_capture_start(config_json: String) -> String {
    let mut config = match controlled_parse_json("controlled_screen_capture_start", &config_json, MAX_CONTROLLED_JSON_BYTES) {
        Ok(value) if value.is_object() => value,
        Ok(_) => return json!({"ok":false,"action":"controlled_screen_capture_start","message":"config must be a JSON object"}).to_string(),
        Err(message) => return json!({"ok":false,"action":"controlled_screen_capture_start","message":message}).to_string(),
    };
    let stream_width = config.get("width").and_then(Value::as_u64).unwrap_or(0);
    let stream_height = config.get("height").and_then(Value::as_u64).unwrap_or(0);
    let width = config
        .get("sourceWidth")
        .and_then(Value::as_u64)
        .unwrap_or(stream_width);
    let height = config
        .get("sourceHeight")
        .and_then(Value::as_u64)
        .unwrap_or(stream_height);
    let display_id = config.get("displayId").and_then(Value::as_u64).unwrap_or(0);
    let frame_rate = config
        .get("frameRate")
        .and_then(Value::as_u64)
        .unwrap_or(30);
    if width == 0
        || height == 0
        || width > 16_384
        || height > 16_384
        || stream_width == 0
        || stream_height == 0
        || stream_width > 16_384
        || stream_height > 16_384
        || display_id > u32::MAX as u64
        || frame_rate == 0
        || frame_rate > 240
    {
        return json!({"ok":false,"action":"controlled_screen_capture_start","message":"width/height must be 1..16384 and frameRate 1..240"}).to_string();
    }
    #[cfg(target_env = "ohos")]
    unsafe {
        if CONTROLLED_CAPTURE_CLEANUP_IN_PROGRESS.load(Ordering::Acquire) {
            return json!({
                "ok": false,
                "action": "controlled_screen_capture_start",
                "message": "previous screen capture cleanup is still in progress"
            })
            .to_string();
        }
        let preparing = CONTROLLED_CAPTURE_START_PREPARING.load(Ordering::Acquire);
        let watched_host_running = controlled_runtime()
            .lock()
            .map(|state| {
                state.running
                    && state.audio_enabled
                    && controlled_view_only_config_is_valid(&state.server_config)
            })
            .unwrap_or(false);
        if !preparing && (!ohos::host_is_started() || !watched_host_running) {
            return json!({
                "ok": false,
                "action": "controlled_screen_capture_start",
                "message": "screen capture is bound to the watched/view-only host lifecycle"
            })
            .to_string();
        }
        let existing = CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire);
        if existing != 0 {
            let healthy = controlled_runtime()
                .lock()
                .map(|state| {
                    state.native_capture_error == 0
                        && state.native_capture_started
                        && state.native_capture_frames > 0
                        && (!state.audio_enabled
                            || CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0)
                })
                .unwrap_or(false);
            if healthy {
                let state = controlled_runtime().lock().unwrap();
                return controlled_response(
                    "controlled_screen_capture_start",
                    true,
                    &state,
                    json!({"idempotent":true,"captureRequested":true}),
                );
            }
            let _ = controlled_screen_capture_stop();
        }
        ohos::reset_host_screen();
        if !ohos::configure_host_screen(stream_width as usize, stream_height as usize) {
            let mut state = controlled_runtime().lock().unwrap();
            state.last_error = Some("Core rejected controlled-host screen geometry".to_string());
            return controlled_response(
                "controlled_screen_capture_start",
                false,
                &state,
                json!({"message":"Core rejected controlled-host screen geometry"}),
            );
        }
        // The regular AVScreenCapture session owns the system privacy prompt.
        // CUSTOM_SCREEN_RECORDING is intentionally not required: it is only an
        // optional restricted permission for suppressing that prompt on
        // supported PC/2in1 products.
        CONTROLLED_CAPTURE_HANDLE.store(CONTROLLED_CAPTURE_LOGICAL_HANDLE, Ordering::Release);
        let configured_size = ohos::host_screen_size();
        if configured_size.0 > 0 && configured_size.1 > 0 {
            config["sourceWidth"] = json!(width);
            config["sourceHeight"] = json!(height);
            config["width"] = json!(configured_size.0);
            config["height"] = json!(configured_size.1);
        }
        {
            let mut state = controlled_runtime().lock().unwrap();
            state.native_capture_state = -1;
            state.native_capture_started = false;
            state.native_capture_error = 0;
            state.native_capture_frames = 0;
            state.native_capture_bytes = 0;
            state.native_capture_audio_frames = 0;
            state.native_capture_audio_bytes = 0;
            state.screenshot_fallback_active = false;
            state.screenshot_fallback_frames = 0;
            state.screenshot_fallback_errors = 0;
            state.screen_config = config;
            state.audio_enabled = true;
            state.last_error = None;
        }
        let audio_enabled = controlled_runtime()
            .lock()
            .map(|state| state.audio_enabled)
            .unwrap_or(false);
        if audio_enabled {
            if let Err(message) = start_controlled_av_capture(
                configured_size.0 as i32,
                configured_size.1 as i32,
                display_id,
                frame_rate as i32,
            ) {
                let _ = controlled_screen_capture_stop();
                let mut state = controlled_runtime().lock().unwrap();
                state.last_error = Some(message.clone());
                return controlled_response(
                    "controlled_screen_capture_start",
                    false,
                    &state,
                    json!({"message":message,"captureRequested":false,"audioEnabled":false}),
                );
            }
        }
        let state = controlled_runtime().lock().unwrap();
        return controlled_response(
            "controlled_screen_capture_start",
            true,
            &state,
            json!({"captureRequested":true,"privacyDialogExpected":true,"framesForwardedToCore":true,"innerAudioCaptureConfigured":audio_enabled,"audioEnabled":audio_enabled,"captureMode":"avscreen_original_stream"}),
        );
    }
    #[cfg(not(target_env = "ohos"))]
    json!({"ok":false,"action":"controlled_screen_capture_start","message":"OH_AVScreenCapture is only available on OHOS"}).to_string()
}

#[napi]
pub fn controlled_screen_capture_stop() -> String {
    #[cfg(target_env = "ohos")]
    unsafe {
        let handle = CONTROLLED_CAPTURE_HANDLE.swap(0, Ordering::AcqRel);
        let audio_result = stop_controlled_av_capture();
        stop_controlled_capture_fallback();
        ohos::stop_host();
        let (audio_stop, audio_release, audio_error) = match audio_result {
            Ok((stop, release)) => (stop, release, None),
            Err(message) => (-1, -1, Some(message)),
        };
        if audio_error.is_some() && CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0 {
            CONTROLLED_CAPTURE_HANDLE.store(CONTROLLED_CAPTURE_LOGICAL_HANDLE, Ordering::Release);
        }
        if handle == 0 {
            let mut state = controlled_runtime().lock().unwrap();
            state.running = false;
            state.audio_enabled = false;
            state.native_capture_started = false;
            if let Some(message) = audio_error.clone() {
                state.last_error = Some(message);
            }
            return controlled_response(
                "controlled_screen_capture_stop",
                audio_error.is_none(),
                &state,
                json!({"idempotent":true,"audioStopCode":audio_stop,"audioReleaseCode":audio_release,"message":audio_error}),
            );
        }
        let (stop, release) = if handle == CONTROLLED_CAPTURE_LOGICAL_HANDLE {
            (0, 0)
        } else {
            let capture = handle as usize as *mut OH_AVScreenCapture;
            (
                OH_AVScreenCapture_StopScreenCapture(capture),
                OH_AVScreenCapture_Release(capture),
            )
        };
        let mut state = controlled_runtime().lock().unwrap();
        state.native_capture_state = -1;
        state.native_capture_started = false;
        state.running = false;
        state.audio_enabled = false;
        if let Some(message) = audio_error.clone() {
            state.last_error = Some(message);
        }
        return controlled_response(
            "controlled_screen_capture_stop",
            stop == 0 && release == 0 && audio_error.is_none(),
            &state,
            json!({"stopCode":stop,"releaseCode":release,"audioStopCode":audio_stop,"audioReleaseCode":audio_release,"message":audio_error}),
        );
    }
    #[cfg(not(target_env = "ohos"))]
    json!({"ok":true,"action":"controlled_screen_capture_stop","idempotent":true}).to_string()
}

#[napi]
pub fn controlled_screen_capture_get_status() -> String {
    let state = controlled_runtime().lock().unwrap();
    let screen_active = CONTROLLED_CAPTURE_HANDLE.load(Ordering::Acquire) != 0;
    let audio_active = CONTROLLED_AUDIO_CAPTURE_HANDLE.load(Ordering::Acquire) != 0;
    let capture_active = screen_active && (!state.audio_enabled || audio_active);
    let capture_healthy = capture_active
        && state.native_capture_error == 0
        && state.native_capture_started
        && state.native_capture_frames > 0;
    controlled_response(
        "controlled_screen_capture_get_status",
        capture_healthy,
        &state,
        json!({
          "available": cfg!(target_env = "ohos"), "active": capture_active,
          "nativeStateCode":state.native_capture_state,"nativeErrorCode":state.native_capture_error,
          "systemCaptureConfirmed":state.native_capture_started,
          "framesObserved":state.native_capture_frames,"bytesObserved":state.native_capture_bytes,
          "audioFramesObserved":state.native_capture_audio_frames,"audioPcmBytesObserved":state.native_capture_audio_bytes,
          "audioFramesForwarded":state.pushed_audio_frames,"audioEnabled":state.audio_enabled,
          "lastTimestampNs":state.native_capture_last_timestamp,"framesForwardedToCore":true,
          "innerAudioCaptureConfigured":audio_active,
          "captureMode":if state.screenshot_fallback_active { "display_pixelmap_fallback" } else { "avscreen" },
          "screenshotFallbackFrames":state.screenshot_fallback_frames,"screenshotFallbackErrors":state.screenshot_fallback_errors
        }),
    )
}

#[napi]
pub fn controlled_input_request_authorization() -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_input_request_authorization");
    }
    #[cfg(target_env = "ohos")]
    unsafe {
        let result = OH_Input_RequestInjection(controlled_input_authorize_callback);
        return json!({"ok":result == 0 || result == 3900005 || result == 3900007,"action":"controlled_input_request_authorization","nativeCode":result,"dialogAuthorizationIndependentOfControlDevice":true}).to_string();
    }
    #[cfg(not(target_env = "ohos"))]
    json!({"ok":false,"action":"controlled_input_request_authorization","message":"input authorization is only available on OHOS"}).to_string()
}

#[napi]
pub fn controlled_input_get_authorization_status() -> String {
    if cfg!(target_env = "ohos") {
        return controlled_view_only_denial("controlled_input_get_authorization_status");
    }
    #[cfg(target_env = "ohos")]
    unsafe {
        let mut status = -1;
        let result = OH_Input_QueryAuthorizedStatus(&mut status);
        if result == 0 {
            CONTROLLED_INPUT_AUTH_STATUS.store(status, Ordering::Release);
        }
        return json!({"ok":result == 0,"action":"controlled_input_get_authorization_status","nativeCode":result,"status":CONTROLLED_INPUT_AUTH_STATUS.load(Ordering::Acquire),"dialogAuthorizationOnly":true}).to_string();
    }
    #[cfg(not(target_env = "ohos"))]
    json!({"ok":false,"action":"controlled_input_get_authorization_status","status":-1}).to_string()
}

#[napi]
pub fn controlled_input_cancel_authorization() -> String {
    #[cfg(target_env = "ohos")]
    unsafe {
        OH_Input_CancelInjection();
        CONTROLLED_INPUT_AUTH_STATUS.store(-1, Ordering::Release);
    }
    json!({"ok":true,"action":"controlled_input_cancel_authorization"}).to_string()
}

#[napi]
pub fn session_get_rgba_size(session_id: String, display: u32) -> u32 {
    let Some(core_session_id) = core_session_id_for(&session_id) else {
        return 0;
    };
    u32::try_from(flutter_ffi::session_get_rgba_size(core_session_id, display as usize).0)
        .unwrap_or(0)
}

#[napi]
pub fn session_take_rgba_frame(
    session_id: String,
    display: u32,
) -> napi_ohos::bindgen_prelude::Uint8Array {
    let Some(core_session_id) = core_session_id_for(&session_id) else {
        return Vec::new().into();
    };
    flutter_ffi::session_take_rgba_frame(core_session_id, display as usize)
        .0
        .into()
}

#[napi]
pub fn session_next_rgba(session_id: String, display: u32) {
    if let Some(core_session_id) = core_session_id_for(&session_id) {
        flutter_ffi::session_next_rgba(core_session_id, display as usize);
    }
}

#[napi]
pub fn session_bind_surface(session_id: String, display: u32, surface_id: String) -> String {
    update_session(&session_id, "session_bind_surface", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let normalized = surface_id.trim();
        let parsed_surface_id = if normalized.is_empty() {
            None
        } else {
            match u64::from_str(normalized) {
                Ok(value) => Some(value),
                Err(err) => {
                    return (false, format!("Invalid surface id {}: {}", normalized, err));
                }
            }
        };
        set_surface_binding(&session.normalized_peer_id, display, parsed_surface_id);
        session.last_error = None;
        if let Some(surface_id) = parsed_surface_id {
            (
                true,
                format!("Bound display {} to surface {}", display, surface_id),
            )
        } else {
            (true, format!("Cleared display {} surface binding", display))
        }
    })
}

#[napi]
pub fn session_refresh(session_id: String, display: u32) -> String {
    update_session(&session_id, "session_refresh", |session| {
        if session.phase == "closed" {
            return (false, "Session is closed".to_string());
        }
        let Some(core_session_id) = parse_core_session_id(session) else {
            return (false, "Missing core session id".to_string());
        };
        flutter_ffi::session_refresh(core_session_id, display as usize);
        session.last_error = None;
        (
            true,
            format!("Requested RustDesk video refresh for display {}", display),
        )
    })
}

#[napi]
pub fn session_get_render_stats(session_id: String, display: u32) -> String {
    let sessions = session_store().lock().unwrap();
    let Some(session) = sessions.get(&session_id) else {
        return action_response(
            "session_get_render_stats",
            false,
            format!("Session {} was not found", session_id),
            None,
        );
    };
    let Some(core_session_id) = parse_core_session_id(session) else {
        return action_response(
            "session_get_render_stats",
            false,
            "Missing core session id".to_string(),
            Some(session),
        );
    };
    let stats = render_stats(&core_session_id.to_string(), display as usize);
    json!({
      "ok": true,
      "action": "session_get_render_stats",
      "stats": stats,
      "session": session_value(session),
      "upstream": upstream_status_value()
    })
    .to_string()
}

#[napi]
pub fn session_update_supported_decodings(session_id: String) -> String {
    update_session(
        &session_id,
        "session_update_supported_decodings",
        |session| {
            if session.phase == "closed" {
                return (false, "Session is closed".to_string());
            }
            let Some(core_session_id) = parse_core_session_id(session) else {
                return (false, "Missing core session id".to_string());
            };
            flutter_ffi::session_change_prefer_codec(core_session_id);
            session.last_error = None;
            (
                true,
                "Pushed updated supported decodings to RustDesk peer".to_string(),
            )
        },
    )
}

#[napi]
pub fn session_get(session_id: String) -> String {
    let sessions = session_store().lock().unwrap();
    match sessions.get(&session_id) {
        Some(session) => action_response(
            "session_get",
            true,
            "Session found".to_string(),
            Some(session),
        ),
        None => action_response(
            "session_get",
            false,
            format!("Session {} was not found", session_id),
            None,
        ),
    }
}

#[napi]
pub fn session_list() -> String {
    let sessions = session_store().lock().unwrap();
    let values = sessions.values().map(session_value).collect::<Vec<_>>();
    json!({
      "ok": true,
      "action": "session_list",
      "sessions": values,
      "upstream": upstream_status_value()
    })
    .to_string()
}

fn session_store() -> &'static Mutex<HashMap<String, BridgeSession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn update_session(
    session_id: &str,
    action: &str,
    op: impl FnOnce(&mut BridgeSession) -> (bool, String),
) -> String {
    let mut sessions = session_store().lock().unwrap();
    let Some(session) = sessions.get_mut(session_id) else {
        return action_response(
            action,
            false,
            format!("Session {} was not found", session_id),
            None,
        );
    };

    session.last_action = action.to_string();
    let (ok, message) = op(session);
    action_response(action, ok, message, Some(session))
}

fn action_response(
    action: &str,
    ok: bool,
    message: String,
    session: Option<&BridgeSession>,
) -> String {
    json!({
      "ok": ok,
      "action": action,
      "message": message,
      "session": session.map(session_value),
      "upstream": upstream_status_value()
    })
    .to_string()
}

fn session_value(session: &BridgeSession) -> Value {
    json!({
      "sessionId": session.session_id,
      "coreSessionId": session.core_session_id,
      "peerTarget": session.peer_target,
      "normalizedPeerId": session.normalized_peer_id,
      "customServer": session.custom_server,
      "serverKey": session.server_key,
      "relaySuffixRequested": session.relay_suffix_requested,
      "forceRelay": session.force_relay,
      "connType": session.conn_type,
      "viewOnly": session.view_only,
      "phase": session.phase,
      "lastAction": session.last_action,
      "lastError": session.last_error,
      "passwordPresent": session.password_present,
      "sharedPassword": session.shared_password,
      "passwordEphemeral": session.password_ephemeral,
      "rememberRequested": session.remember_requested,
      "twoFactorPending": session.two_factor_pending,
      "selectedDisplays": session.selected_displays,
      "switchUuid": session.switch_uuid,
      "connTokenPresent": session.conn_token_present,
      "lastPointerPayload": session.last_pointer_payload,
      "lastKeyPayload": session.last_key_payload,
      "lastTextPayload": session.last_text_payload,
      "coreBindingAvailable": core_binding_ready()
    })
}

fn parse_json_payload(raw: &str, name: &str) -> Result<Value, String> {
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }

    serde_json::from_str::<Value>(raw).map_err(|err| format!("Invalid {}: {}", name, err))
}

fn make_core_session_id(requested: &str) -> flutter_ffi::SessionID {
    flutter_ffi::SessionID::from_str(requested).unwrap_or_else(|_| flutter_ffi::SessionID::new_v4())
}

fn parse_core_session_id(session: &BridgeSession) -> Option<flutter_ffi::SessionID> {
    session
        .core_session_id
        .as_ref()
        .and_then(|value| flutter_ffi::SessionID::from_str(value).ok())
}

fn make_session_id(requested: &str) -> String {
    if !requested.trim().is_empty() {
        return requested.trim().to_string();
    }

    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("hm-session-{:016x}", counter)
}

fn connection_type_from_options(options: &Value) -> &'static str {
    if json_bool(options, &["isFileTransfer", "is_file_transfer"]) {
        "file_transfer"
    } else if json_bool(options, &["isViewCamera", "is_view_camera"]) {
        "view_camera"
    } else if json_bool(options, &["isTerminal", "is_terminal"]) {
        "terminal"
    } else if json_bool(options, &["isPortForward", "is_port_forward"]) {
        if json_bool(options, &["isRdp", "is_rdp"]) {
            "rdp"
        } else {
            "port_forward"
        }
    } else {
        "default_conn"
    }
}

fn parse_display_list(value: &Value) -> Vec<i32> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(|item| item.as_i64().and_then(|number| i32::try_from(number).ok()))
            .collect();
    }

    value
        .get("displays")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_i64().and_then(|number| i32::try_from(number).ok()))
                .collect()
        })
        .unwrap_or_default()
}

fn json_lookup<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

fn json_bool(value: &Value, keys: &[&str]) -> bool {
    match json_lookup(value, keys) {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => matches!(v.as_str(), "1" | "true" | "TRUE" | "Y" | "y" | "on"),
        Some(Value::Number(v)) => v.as_i64().unwrap_or_default() != 0,
        _ => false,
    }
}

fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    json_lookup(value, keys)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn json_raw_string(value: &Value, keys: &[&str]) -> Option<String> {
    json_lookup(value, keys)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_i32(value: &Value, keys: &[&str]) -> Option<i32> {
    match json_lookup(value, keys)? {
        Value::Number(v) => v.as_i64().and_then(|number| i32::try_from(number).ok()),
        Value::String(v) => v.trim().parse::<i32>().ok(),
        _ => None,
    }
}

fn json_field_string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn json_array_field(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .filter(|value| value.is_array())
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn json_object_field(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}

fn job_start_error(action: &str, message: String) -> String {
    json!({
      "ok": false,
      "action": action,
      "state": "failed",
      "message": message
    })
    .to_string()
}

fn background_job_failure(action: &str, category: &str, message: String) -> Value {
    json!({
      "ok": false,
      "action": action,
      "state": "failed",
      "category": category,
      "message": message
    })
}

fn start_background_job<F>(
    store: &'static Mutex<BackgroundJsonJob>,
    action: &'static str,
    task: F,
) -> String
where
    F: FnOnce(u64) -> Value + Send + 'static,
{
    let mut job = match store.lock() {
        Ok(job) => job,
        Err(_) => {
            return job_start_error(action, "Background job state is unavailable".to_string());
        }
    };
    if job.running {
        return job_start_error(action, "Another operation is already running".to_string());
    }
    job.running = true;
    job.result = None;
    job.generation = job.generation.wrapping_add(1);
    let generation = job.generation;
    drop(job);

    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(generation)))
            .unwrap_or_else(|_| {
                background_job_failure(
                    action,
                    "internal",
                    "Background operation failed".to_string(),
                )
            });
        if let Ok(mut job) = store.lock() {
            if job.generation != generation {
                return;
            }
            job.running = false;
            job.result = Some(result);
        }
    });

    json!({
      "ok": true,
      "action": action,
      "state": "pending"
    })
    .to_string()
}

fn background_job_is_current(store: &'static Mutex<BackgroundJsonJob>, generation: u64) -> bool {
    store
        .lock()
        .map(|job| job.running && job.generation == generation)
        .unwrap_or(false)
}

fn cancel_background_job(store: &'static Mutex<BackgroundJsonJob>) {
    if let Ok(mut job) = store.lock() {
        job.generation = job.generation.wrapping_add(1);
        job.running = false;
        job.result = None;
    }
}

fn poll_background_job(store: &'static Mutex<BackgroundJsonJob>, action: &str) -> String {
    let mut job = match store.lock() {
        Ok(job) => job,
        Err(_) => {
            return job_start_error(action, "Background job state is unavailable".to_string());
        }
    };
    if job.running {
        return json!({
          "ok": true,
          "action": action,
          "state": "pending"
        })
        .to_string();
    }
    if let Some(result) = job.result.take() {
        return result.to_string();
    }
    json!({
      "ok": true,
      "action": action,
      "state": "idle"
    })
    .to_string()
}

fn account_user_summary(user: &Value) -> Value {
    json!({
      "name": json_field_string(user, "name"),
      "displayName": json_field_string(user, "display_name"),
      "email": json_field_string(user, "email"),
      "status": user.get("status").and_then(Value::as_i64).unwrap_or(1),
      "isAdmin": user.get("is_admin").and_then(Value::as_bool).unwrap_or(false)
    })
}

fn account_login_options() -> Value {
    let api_server = configured_api_server();
    if api_server.trim().is_empty() {
        return json!({
          "ok": false,
          "action": "runtime_get_account_login_options",
          "state": "failed",
          "message": "Account API is not configured",
          "providers": []
        });
    }
    let response = match api_request(&api_server, "/api/login-options", "GET", None, None) {
        Ok(response) => response,
        Err(message) => {
            return json!({
              "ok": false,
              "action": "runtime_get_account_login_options",
              "state": "failed",
              "message": message,
              "providers": []
            });
        }
    };
    let body = match require_api_success(response, "Unable to load account login options") {
        Ok(body) => body,
        Err(message) => {
            return json!({
              "ok": false,
              "action": "runtime_get_account_login_options",
              "state": "failed",
              "message": message,
              "providers": []
            });
        }
    };
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    if let Some(options) = body.as_array() {
        for option in options.iter().filter_map(Value::as_str) {
            if let Some(raw_common) = option.strip_prefix("common-oidc/") {
                if let Ok(common_options) = serde_json::from_str::<Value>(raw_common) {
                    if let Some(common_options) = common_options.as_array() {
                        for common_option in common_options {
                            let name = json_field_string(common_option, "name").trim().to_string();
                            let identity = name.to_ascii_lowercase();
                            if !name.is_empty() && seen.insert(identity) {
                                names.push(name);
                            }
                        }
                    }
                }
            } else if let Some(provider) = option.strip_prefix("oidc/") {
                let name = provider.trim().to_string();
                let identity = name.to_ascii_lowercase();
                if !name.is_empty() && seen.insert(identity) {
                    names.push(name);
                }
            }
        }
    }
    let providers = names
        .into_iter()
        .map(|name| {
            let label = match name.to_ascii_lowercase().as_str() {
                "github" => "GitHub".to_string(),
                "google" => "Google".to_string(),
                "microsoft" | "azure" => "Microsoft".to_string(),
                _ => {
                    let mut chars = name.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => name.clone(),
                    }
                }
            };
            json!({ "name": name, "label": label })
        })
        .collect::<Vec<_>>();
    json!({
      "ok": true,
      "action": "runtime_get_account_login_options",
      "state": "ready",
      "apiServer": api_server,
      "providers": providers
    })
}

fn account_state_snapshot() -> Value {
    let api_server = configured_api_server();
    let access_token = flutter_ffi::main_get_local_option("access_token".to_string()).0;
    let user_info = flutter_ffi::main_get_local_option("user_info".to_string()).0;
    let user = serde_json::from_str::<Value>(&user_info).unwrap_or_else(|_| json!({}));
    json!({
      "ok": true,
      "action": "runtime_get_account_state",
      "loggedIn": !api_server.is_empty() && !access_token.is_empty(),
      "apiServer": api_server,
      "user": account_user_summary(&user)
    })
}

fn clear_account_state() {
    flutter_ffi::main_set_local_option("access_token".to_string(), String::new());
    flutter_ffi::main_set_local_option("user_info".to_string(), String::new());
    flutter_ffi::main_clear_ab();
    flutter_ffi::main_clear_group();
    if let Ok(mut challenge) = account_challenge_store().lock() {
        *challenge = None;
    }
}

fn api_request_headers(access_token: Option<&str>, empty_body: bool) -> String {
    let mut headers = serde_json::Map::new();
    headers.insert("Content-Type".to_string(), "application/json".into());
    if empty_body {
        headers.insert("Content-Length".to_string(), "0".into());
    }
    if let Some(access_token) = access_token.filter(|token| !token.is_empty()) {
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", access_token).into(),
        );
    }
    Value::Object(headers).to_string()
}

fn api_request(
    api_server: &str,
    path: &str,
    method: &str,
    body: Option<String>,
    access_token: Option<&str>,
) -> Result<ApiResponse, String> {
    let url = format!(
        "{}/{}",
        api_server.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    let headers = api_request_headers(access_token, body.is_none());
    let raw = librustdesk::common::http_request_sync(url, method.to_string(), body, headers)
        .map_err(|err| err.to_string())?;
    let envelope = serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("Invalid HTTP response envelope: {}", err))?;
    let status_code = envelope
        .get("status_code")
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| "HTTP response has no status code".to_string())?;
    let raw_body = envelope
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let body = if raw_body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(&raw_body).unwrap_or_else(|_| Value::String(raw_body.clone()))
    };
    Ok(ApiResponse { status_code, body })
}

fn api_error_message(response: &ApiResponse) -> String {
    let message = response
        .body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if !message.is_empty() {
        return message.to_string();
    }
    if let Value::String(body) = &response.body {
        let body = body.trim();
        if !body.is_empty() {
            return format!("HTTP {}: {}", response.status_code, body);
        }
    }
    format!("HTTP {}", response.status_code)
}

fn require_api_success(response: ApiResponse, context: &str) -> Result<Value, String> {
    if !(200..300).contains(&response.status_code) {
        return Err(format!("{}: {}", context, api_error_message(&response)));
    }
    if response.body.get("error").is_some() {
        return Err(format!("{}: {}", context, api_error_message(&response)));
    }
    Ok(response.body)
}

fn address_book_sync_is_current(api_server: &str, access_token: &str, generation: u64) -> bool {
    background_job_is_current(address_book_job_store(), generation)
        && configured_api_server() == api_server
        && flutter_ffi::main_get_local_option("access_token".to_string()).0 == access_token
}

fn require_address_book_success(
    response: ApiResponse,
    context: &str,
    api_server: &str,
    access_token: &str,
    generation: u64,
) -> Result<Value, String> {
    if response.status_code == 401 {
        if let Ok(job) = address_book_job_store().lock() {
            if job.running
                && job.generation == generation
                && configured_api_server() == api_server
                && flutter_ffi::main_get_local_option("access_token".to_string()).0 == access_token
            {
                clear_account_state();
            }
        }
    }
    require_api_success(response, context)
}

fn login_request_body(username: &str, password: Option<&str>) -> Value {
    let device_info_raw = flutter_ffi::main_get_login_device_info().0;
    let device_info = serde_json::from_str::<Value>(&device_info_raw).unwrap_or_else(|_| json!({}));
    let mut body = serde_json::Map::new();
    body.insert("username".to_string(), username.into());
    if let Some(password) = password {
        body.insert("password".to_string(), password.into());
    }
    body.insert("id".to_string(), flutter_ffi::main_get_my_id().into());
    body.insert("uuid".to_string(), flutter_ffi::main_get_uuid().into());
    body.insert("autoLogin".to_string(), true.into());
    body.insert("type".to_string(), "account".into());
    body.insert("deviceInfo".to_string(), device_info);
    Value::Object(body)
}

fn verification_request_body(challenge: &AccountChallenge, code: &str) -> Value {
    let device_info_raw = flutter_ffi::main_get_login_device_info().0;
    let device_info = serde_json::from_str::<Value>(&device_info_raw).unwrap_or_else(|_| json!({}));
    let mut body = serde_json::Map::new();
    body.insert("verificationCode".to_string(), code.into());
    if challenge.challenge_type == "totp" {
        body.insert("tfaCode".to_string(), code.into());
    }
    body.insert("secret".to_string(), challenge.secret.clone().into());
    body.insert("username".to_string(), challenge.username.clone().into());
    body.insert("id".to_string(), flutter_ffi::main_get_my_id().into());
    body.insert("uuid".to_string(), flutter_ffi::main_get_uuid().into());
    body.insert("autoLogin".to_string(), true.into());
    body.insert("type".to_string(), "email_code".into());
    body.insert("deviceInfo".to_string(), device_info);
    Value::Object(body)
}

fn handle_account_response(
    response: ApiResponse,
    api_server: &str,
    fallback_username: &str,
    generation: u64,
) -> Value {
    if !background_job_is_current(account_job_store(), generation)
        || configured_api_server() != api_server
    {
        return background_job_failure(
            "runtime_poll_account_action",
            "superseded",
            "Server configuration changed while signing in".to_string(),
        );
    }
    if !(200..300).contains(&response.status_code) {
        return background_job_failure(
            "runtime_poll_account_action",
            "http",
            api_error_message(&response),
        );
    }
    if response.body.get("error").is_some() {
        return background_job_failure(
            "runtime_poll_account_action",
            "server",
            api_error_message(&response),
        );
    }
    let response_type = json_field_string(&response.body, "type");
    if response_type == "access_token" {
        let access_token = json_field_string(&response.body, "access_token");
        if access_token.is_empty() {
            return background_job_failure(
                "runtime_poll_account_action",
                "protocol",
                "Login response contains no access token".to_string(),
            );
        }
        let user = response
            .body
            .get("user")
            .filter(|user| user.is_object())
            .cloned()
            .unwrap_or_else(|| json!({ "name": fallback_username }));
        let account_job = match account_job_store().lock() {
            Ok(job) if job.running && job.generation == generation => job,
            Ok(_) => {
                return background_job_failure(
                    "runtime_poll_account_action",
                    "superseded",
                    "Server configuration changed while signing in".to_string(),
                );
            }
            Err(_) => {
                return background_job_failure(
                    "runtime_poll_account_action",
                    "internal",
                    "Account job state is unavailable".to_string(),
                );
            }
        };
        if configured_api_server() != api_server {
            return background_job_failure(
                "runtime_poll_account_action",
                "superseded",
                "Server configuration changed while signing in".to_string(),
            );
        }
        flutter_ffi::main_set_local_option("access_token".to_string(), access_token);
        flutter_ffi::main_set_local_option("user_info".to_string(), user.to_string());
        if let Ok(mut challenge) = account_challenge_store().lock() {
            *challenge = None;
        }
        drop(account_job);
        return json!({
          "ok": true,
          "action": "runtime_poll_account_action",
          "state": "authenticated",
          "user": account_user_summary(&user)
        });
    }

    if response_type == "email_check" || response_type == "tfa_check" {
        let tfa_type = json_field_string(&response.body, "tfa_type");
        let challenge_type = if response_type == "tfa_check" || tfa_type == "tfa_check" {
            "totp"
        } else if tfa_type.is_empty() || tfa_type == "email_check" {
            "email"
        } else {
            "unsupported"
        };
        if challenge_type == "unsupported" {
            return background_job_failure(
                "runtime_poll_account_action",
                "protocol",
                "The server requested an unsupported verification method".to_string(),
            );
        }
        let user = response
            .body
            .get("user")
            .filter(|user| user.is_object())
            .cloned()
            .unwrap_or_else(|| json!({ "name": fallback_username }));
        let username = json_field_string(&user, "name");
        let username = if username.is_empty() {
            fallback_username.to_string()
        } else {
            username
        };
        let account_job = match account_job_store().lock() {
            Ok(job) if job.running && job.generation == generation => job,
            Ok(_) => {
                return background_job_failure(
                    "runtime_poll_account_action",
                    "superseded",
                    "Server configuration changed while signing in".to_string(),
                );
            }
            Err(_) => {
                return background_job_failure(
                    "runtime_poll_account_action",
                    "internal",
                    "Account job state is unavailable".to_string(),
                );
            }
        };
        if configured_api_server() != api_server {
            return background_job_failure(
                "runtime_poll_account_action",
                "superseded",
                "Server configuration changed while signing in".to_string(),
            );
        }
        let challenge_id = format!(
            "account-challenge-{}",
            ACCOUNT_CHALLENGE_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let challenge = AccountChallenge {
            id: challenge_id.clone(),
            api_server: api_server.to_string(),
            username,
            secret: json_field_string(&response.body, "secret"),
            challenge_type: challenge_type.to_string(),
        };
        if let Ok(mut stored) = account_challenge_store().lock() {
            *stored = Some(challenge);
        }
        drop(account_job);
        return json!({
          "ok": true,
          "action": "runtime_poll_account_action",
          "state": "challenge",
          "challengeId": challenge_id,
          "challengeType": challenge_type,
          "emailHint": json_field_string(&user, "email")
        });
    }

    background_job_failure(
        "runtime_poll_account_action",
        "protocol",
        "Unexpected login response from server".to_string(),
    )
}

fn start_account_login_job(username: String, password: String) -> String {
    let api_server = configured_api_server();
    if api_server.trim().is_empty() {
        return job_start_error(
            "runtime_start_account_login",
            "Account API is not configured".to_string(),
        );
    }
    cancel_background_job(address_book_job_store());
    flutter_ffi::main_account_auth_cancel();
    if let Ok(mut challenge) = account_challenge_store().lock() {
        *challenge = None;
    }
    start_background_job(
        account_job_store(),
        "runtime_start_account_login",
        move |generation| {
            let body = login_request_body(&username, Some(&password)).to_string();
            match api_request(&api_server, "/api/login", "POST", Some(body), None) {
                Ok(response) => {
                    handle_account_response(response, &api_server, &username, generation)
                }
                Err(message) => {
                    background_job_failure("runtime_poll_account_action", "transport", message)
                }
            }
        },
    )
}

fn start_account_verification_job(challenge_id: String, code: String) -> String {
    let challenge = match account_challenge_store().lock() {
        Ok(challenge) => challenge
            .as_ref()
            .filter(|challenge| challenge.id == challenge_id)
            .cloned(),
        Err(_) => None,
    };
    let Some(challenge) = challenge else {
        return job_start_error(
            "runtime_start_account_verification",
            "The verification challenge has expired".to_string(),
        );
    };
    start_background_job(
        account_job_store(),
        "runtime_start_account_verification",
        move |generation| {
            let body = verification_request_body(&challenge, &code).to_string();
            match api_request(
                &challenge.api_server,
                "/api/login",
                "POST",
                Some(body),
                None,
            ) {
                Ok(response) => handle_account_response(
                    response,
                    &challenge.api_server,
                    &challenge.username,
                    generation,
                ),
                Err(message) => {
                    background_job_failure("runtime_poll_account_action", "transport", message)
                }
            }
        },
    )
}

fn account_logout() -> String {
    let api_server = configured_api_server();
    let access_token = flutter_ffi::main_get_local_option("access_token".to_string()).0;
    let body = json!({
      "id": flutter_ffi::main_get_my_id(),
      "uuid": flutter_ffi::main_get_uuid()
    })
    .to_string();
    cancel_background_job(account_job_store());
    cancel_background_job(account_options_job_store());
    cancel_background_job(address_book_job_store());
    flutter_ffi::main_account_auth_cancel();
    clear_account_state();
    if !api_server.trim().is_empty() && !access_token.is_empty() {
        std::thread::spawn(move || {
            let _ = api_request(
                &api_server,
                "/api/logout",
                "POST",
                Some(body),
                Some(&access_token),
            );
        });
    }
    json!({
      "ok": true,
      "action": "runtime_account_logout",
      "state": "logged_out"
    })
    .to_string()
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn address_book_peer(peer: &Value, include_hash: bool) -> Option<Value> {
    let id = json_field_string(peer, "id").trim().to_string();
    if id.is_empty() {
        return None;
    }
    let mut result = serde_json::Map::new();
    result.insert("id".to_string(), id.into());
    result.insert(
        "username".to_string(),
        json_field_string(peer, "username").into(),
    );
    result.insert(
        "hostname".to_string(),
        json_field_string(peer, "hostname").into(),
    );
    result.insert(
        "platform".to_string(),
        json_field_string(peer, "platform").into(),
    );
    result.insert("alias".to_string(), json_field_string(peer, "alias").into());
    result.insert("tags".to_string(), json!(string_array(peer.get("tags"))));
    if include_hash {
        let hash = json_field_string(peer, "hash");
        if !hash.is_empty() {
            result.insert("hash".to_string(), hash.into());
        }
    }
    Some(Value::Object(result))
}

fn normalize_tag_colors(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "{}".to_string())
}

fn legacy_address_book_entry(
    response: ApiResponse,
    api_server: &str,
    access_token: &str,
    generation: u64,
) -> Result<Value, String> {
    let body = require_address_book_success(
        response,
        "Unable to load legacy address book",
        api_server,
        access_token,
        generation,
    )?;
    if body.is_null() {
        return Ok(json!({
          "guid": "",
          "name": "Legacy address book",
          "tags": [],
          "peers": [],
          "tag_colors": "{}"
        }));
    }
    let data_raw = body.get("data").and_then(Value::as_str).unwrap_or_default();
    if data_raw.is_empty() {
        return Ok(json!({
          "guid": "",
          "name": "Legacy address book",
          "tags": [],
          "peers": [],
          "tag_colors": "{}"
        }));
    }
    let data = serde_json::from_str::<Value>(data_raw)
        .map_err(|err| format!("Invalid legacy address book: {}", err))?;
    let peers = data
        .get("peers")
        .and_then(Value::as_array)
        .map(|peers| {
            peers
                .iter()
                .filter_map(|peer| address_book_peer(peer, true))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(json!({
      "guid": "",
      "name": "Legacy address book",
      "tags": string_array(data.get("tags")),
      "peers": peers,
      "tag_colors": normalize_tag_colors(&json_field_string(&data, "tag_colors"))
    }))
}

fn fetch_shared_address_book_profiles(
    api_server: &str,
    access_token: &str,
    generation: u64,
) -> Result<Vec<(String, String)>, String> {
    let mut profiles = Vec::new();
    let mut seen = HashSet::new();
    for current in 1..=100usize {
        if !address_book_sync_is_current(api_server, access_token, generation) {
            return Err(ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string());
        }
        let path = format!("/api/ab/shared/profiles?current={}&pageSize=100", current);
        let response = api_request(api_server, &path, "POST", None, Some(access_token))?;
        if response.status_code == 404 {
            return Ok(profiles);
        }
        let body = require_address_book_success(
            response,
            "Unable to load shared address books",
            api_server,
            access_token,
            generation,
        )?;
        let total = body.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
        let rows = body
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut added = 0usize;
        for row in rows.iter() {
            let guid = json_field_string(row, "guid").trim().to_string();
            let name = json_field_string(row, "name").trim().to_string();
            if guid.is_empty() || name.is_empty() || !seen.insert(guid.clone()) {
                continue;
            }
            profiles.push((guid, name));
            added += 1;
        }
        if current * 100 >= total || rows.is_empty() || added == 0 {
            break;
        }
    }
    Ok(profiles)
}

fn fetch_address_book_peers(
    api_server: &str,
    access_token: &str,
    guid: &str,
    include_hash: bool,
    generation: u64,
) -> Result<Vec<Value>, String> {
    let mut peers = Vec::new();
    let mut seen = HashSet::new();
    for current in 1..=100usize {
        if !address_book_sync_is_current(api_server, access_token, generation) {
            return Err(ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string());
        }
        let path = format!("/api/ab/peers?current={}&pageSize=100&ab={}", current, guid);
        let response = api_request(api_server, &path, "POST", None, Some(access_token))?;
        let body = require_address_book_success(
            response,
            "Unable to load address book devices",
            api_server,
            access_token,
            generation,
        )?;
        let total = body.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
        let rows = body
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut added = 0usize;
        for row in rows.iter() {
            let id = json_field_string(row, "id").trim().to_string();
            if id.is_empty() || !seen.insert(id) {
                continue;
            }
            if let Some(peer) = address_book_peer(row, include_hash) {
                peers.push(peer);
                added += 1;
            }
        }
        if current * 100 >= total || rows.is_empty() || added == 0 {
            break;
        }
    }
    Ok(peers)
}

fn fetch_address_book_tags(
    api_server: &str,
    access_token: &str,
    guid: &str,
    generation: u64,
) -> Result<(Vec<String>, String), String> {
    if !address_book_sync_is_current(api_server, access_token, generation) {
        return Err(ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string());
    }
    let path = format!("/api/ab/tags/{}", guid);
    let response = api_request(api_server, &path, "POST", None, Some(access_token))?;
    let body = require_address_book_success(
        response,
        "Unable to load address book tags",
        api_server,
        access_token,
        generation,
    )?;
    let rows = body
        .as_array()
        .cloned()
        .ok_or_else(|| "Invalid address book tag list".to_string())?;
    let mut tags = Vec::new();
    let mut colors = serde_json::Map::new();
    let mut seen = HashSet::new();
    for row in rows.iter() {
        let name = json_field_string(row, "name").trim().to_string();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        tags.push(name.clone());
        if let Some(color) = row.get("color").filter(|color| color.is_number()) {
            colors.insert(name, color.clone());
        }
    }
    Ok((tags, Value::Object(colors).to_string()))
}

fn sync_address_book(api_server: String, access_token: String, generation: u64) -> Value {
    if !address_book_sync_is_current(&api_server, &access_token, generation) {
        return background_job_failure(
            "runtime_poll_address_book_sync",
            "superseded",
            ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string(),
        );
    }
    let personal_response = match api_request(
        &api_server,
        "/api/ab/personal",
        "POST",
        None,
        Some(&access_token),
    ) {
        Ok(response) => response,
        Err(message) => {
            return background_job_failure("runtime_poll_address_book_sync", "transport", message);
        }
    };

    let entries = if personal_response.status_code == 404 {
        if !address_book_sync_is_current(&api_server, &access_token, generation) {
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "superseded",
                ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string(),
            );
        }
        match api_request(&api_server, "/api/ab", "GET", None, Some(&access_token)).and_then(
            |response| legacy_address_book_entry(response, &api_server, &access_token, generation),
        ) {
            Ok(entry) => vec![entry],
            Err(message) => {
                return background_job_failure("runtime_poll_address_book_sync", "server", message);
            }
        }
    } else {
        let personal_body = match require_address_book_success(
            personal_response,
            "Unable to load personal address book",
            &api_server,
            &access_token,
            generation,
        ) {
            Ok(body) => body,
            Err(message) => {
                return background_job_failure("runtime_poll_address_book_sync", "server", message);
            }
        };
        let personal_guid = json_field_string(&personal_body, "guid").trim().to_string();
        if personal_guid.is_empty() {
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "protocol",
                "Personal address book has no guid".to_string(),
            );
        }
        let mut profiles = vec![(personal_guid.clone(), "My address book".to_string(), true)];
        match fetch_shared_address_book_profiles(&api_server, &access_token, generation) {
            Ok(shared) => {
                for (guid, name) in shared {
                    if guid != personal_guid {
                        profiles.push((guid, name, false));
                    }
                }
            }
            Err(message) => {
                return background_job_failure("runtime_poll_address_book_sync", "server", message);
            }
        }

        let mut entries = Vec::new();
        for (guid, name, personal) in profiles {
            if !address_book_sync_is_current(&api_server, &access_token, generation) {
                return background_job_failure(
                    "runtime_poll_address_book_sync",
                    "superseded",
                    ADDRESS_BOOK_SUPERSEDED_MESSAGE.to_string(),
                );
            }
            let peers = match fetch_address_book_peers(
                &api_server,
                &access_token,
                &guid,
                personal,
                generation,
            ) {
                Ok(peers) => peers,
                Err(message) => {
                    return background_job_failure(
                        "runtime_poll_address_book_sync",
                        "server",
                        message,
                    );
                }
            };
            let (tags, tag_colors) =
                match fetch_address_book_tags(&api_server, &access_token, &guid, generation) {
                    Ok(tags) => tags,
                    Err(message) => {
                        return background_job_failure(
                            "runtime_poll_address_book_sync",
                            "server",
                            message,
                        );
                    }
                };
            entries.push(json!({
              "guid": guid,
              "name": name,
              "tags": tags,
              "peers": peers,
              "tag_colors": tag_colors
            }));
        }
        entries
    };

    let address_book_job = match address_book_job_store().lock() {
        Ok(job) if job.running && job.generation == generation => job,
        Ok(_) => {
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "superseded",
                "Server or account changed while synchronizing".to_string(),
            );
        }
        Err(_) => {
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "internal",
                "Address book job state is unavailable".to_string(),
            );
        }
    };
    if configured_api_server() != api_server
        || flutter_ffi::main_get_local_option("access_token".to_string()).0 != access_token
    {
        return background_job_failure(
            "runtime_poll_address_book_sync",
            "superseded",
            "Server or account changed while synchronizing".to_string(),
        );
    }
    let cache = json!({
      "access_token": access_token,
      "ab_entries": entries
    });
    flutter_ffi::main_save_ab(cache.to_string());
    drop(address_book_job);
    let peer_count = entries
        .iter()
        .map(|entry| {
            entry
                .get("peers")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .sum::<usize>();
    json!({
      "ok": true,
      "action": "runtime_poll_address_book_sync",
      "state": "synced",
      "addressBookCount": entries.len(),
      "peerCount": peer_count
    })
}

fn start_address_book_sync_job() -> String {
    let api_server = configured_api_server();
    let access_token = flutter_ffi::main_get_local_option("access_token".to_string()).0;
    if api_server.trim().is_empty() || access_token.is_empty() {
        return job_start_error(
            "runtime_start_address_book_sync",
            "Sign in before synchronizing the address book".to_string(),
        );
    }
    start_background_job(
        address_book_job_store(),
        "runtime_start_address_book_sync",
        move |generation| sync_address_book(api_server, access_token, generation),
    )
}

fn normalize_server_endpoint(value: String) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn configured_id_server() -> String {
    flutter_ffi::main_get_option("custom-rendezvous-server".to_string())
        .trim()
        .to_string()
}

fn configured_api_server() -> String {
    let id_server = configured_id_server();
    let api_server = flutter_ffi::main_get_option("api-server".to_string());
    if id_server.is_empty() && api_server.trim().is_empty() {
        String::new()
    } else {
        flutter_ffi::main_get_api_server()
    }
}

fn server_config_snapshot() -> Value {
    let options = serde_json::from_str::<Value>(&flutter_ffi::main_get_options())
        .unwrap_or_else(|_| json!({}));
    let id_server = json_field_string(&options, "custom-rendezvous-server");
    let relay_server = json_field_string(&options, "relay-server");
    let api_server = json_field_string(&options, "api-server");
    let key = json_field_string(&options, "key");
    let effective_api_server = configured_api_server();
    let account_available = !effective_api_server.is_empty()
        && !flutter_ffi::main_get_local_option("access_token".to_string())
            .0
            .is_empty();
    json!({
      "mode": "custom",
      "idServer": id_server,
      "relayServer": relay_server,
      "apiServer": api_server,
      "key": key,
      "effectiveApiServer": effective_api_server,
      "usingPublicServer": false,
      "accountAvailable": account_available
    })
}

fn parse_server_config(raw: &str) -> Result<Value, String> {
    if raw.trim().is_empty() {
        return Err("Server configuration is empty".to_string());
    }
    let payload = parse_json_payload(raw, "server configuration")?;
    if !payload.is_object() {
        return Err("Server configuration must be a JSON object".to_string());
    }
    let requested_mode = json_raw_string(&payload, &["mode"])
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if requested_mode == "official" {
        return Err(
            "Official server mode is not available; configure the server explicitly".to_string(),
        );
    }
    if !requested_mode.is_empty() && requested_mode != "custom" {
        return Err(format!("Unsupported server mode {}", requested_mode));
    }
    let id_server = normalize_server_endpoint(
        json_raw_string(&payload, &["idServer", "customRendezvousServer", "host"])
            .unwrap_or_default(),
    );
    let relay_server = normalize_server_endpoint(
        json_raw_string(&payload, &["relayServer", "relay"]).unwrap_or_default(),
    );
    let api_server = normalize_server_endpoint(
        json_raw_string(&payload, &["apiServer", "api"]).unwrap_or_default(),
    );
    let key = json_raw_string(&payload, &["key"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let test_with_proxy = json_lookup(&payload, &["testWithProxy", "test_with_proxy"])
        .map(|_| json_bool(&payload, &["testWithProxy", "test_with_proxy"]))
        .unwrap_or(true);
    Ok(json!({
      "mode": "custom",
      "idServer": id_server,
      "relayServer": relay_server,
      "apiServer": api_server,
      "key": key,
      "testWithProxy": test_with_proxy
    }))
}

fn validate_server_config_format(config: &Value) -> Value {
    let id_server = json_field_string(config, "idServer");
    let relay_server = json_field_string(config, "relayServer");
    let api_server = json_field_string(config, "apiServer");
    let id_error = if id_server.is_empty() {
        String::new()
    } else {
        validate_socket_target(&id_server, hbb_common::config::RENDEZVOUS_PORT)
            .err()
            .unwrap_or_default()
    };
    let relay_error = if relay_server.is_empty() {
        String::new()
    } else {
        validate_socket_target(&relay_server, hbb_common::config::RELAY_PORT)
            .err()
            .unwrap_or_default()
    };
    let api_error = if api_server.is_empty() {
        String::new()
    } else {
        api_socket_target(&api_server)
            .and_then(|target| validate_normalized_socket_target(&target).map(|_| target))
            .err()
            .unwrap_or_default()
    };
    json!({
      "idServer": id_error,
      "relayServer": relay_error,
      "apiServer": api_error
    })
}

async fn test_server_config(config: &Value) -> Value {
    let format_errors = validate_server_config_format(config);
    if !server_config_errors_empty(&format_errors) {
        return format_errors;
    }
    let id_server = json_field_string(config, "idServer");
    let relay_server = json_field_string(config, "relayServer");
    let api_server = json_field_string(config, "apiServer");
    let key = json_field_string(config, "key");
    let test_with_proxy = config
        .get("testWithProxy")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let id_target = if id_server.is_empty() {
        None
    } else {
        Some(socket_target(
            &id_server,
            hbb_common::config::RENDEZVOUS_PORT as u16,
        ))
    };
    let relay_target = if relay_server.is_empty() {
        None
    } else {
        Some(socket_target(
            &relay_server,
            hbb_common::config::RELAY_PORT as u16,
        ))
    };
    let api_url = if api_server.is_empty() {
        None
    } else {
        Some(api_server)
    };
    let (id_error, relay_error, api_error) = hbb_common::tokio::join!(
        test_id_server_response(id_target, test_with_proxy),
        test_relay_server_response(relay_target, key, test_with_proxy),
        test_api_server_response(api_url, test_with_proxy),
    );
    json!({
      "idServer": id_error,
      "relayServer": relay_error,
      "apiServer": api_error
    })
}

fn server_config_has_endpoint(config: &Value) -> bool {
    !json_field_string(config, "idServer").is_empty()
        || !json_field_string(config, "relayServer").is_empty()
        || !json_field_string(config, "apiServer").is_empty()
}

fn socket_target(value: &str, default_port: u16) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return format!("{trimmed}:{default_port}");
    }
    hbb_common::socket_client::check_port(trimmed, default_port as i32)
}

fn validate_socket_target(value: &str, default_port: i32) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("缺少服务器地址".to_string());
    }
    if trimmed.contains(char::is_whitespace) {
        return Err("地址不能包含空白字符".to_string());
    }
    let target = socket_target(trimmed, default_port as u16);
    validate_normalized_socket_target(&target)?;
    Ok(target)
}

fn validate_normalized_socket_target(target: &str) -> Result<(), String> {
    let Some((host, port)) = hbb_common::socket_client::split_host_port(target) else {
        return Err("需要有效的主机名和端口".to_string());
    };
    let host = host.trim_matches(['[', ']']).trim();
    if host.is_empty()
        || host
            .chars()
            .any(|character| matches!(character, '/' | '@' | '?' | '#'))
        || port <= 0
        || port > u16::MAX as i32
    {
        return Err("需要有效的主机名和端口".to_string());
    }
    Ok(())
}

fn api_socket_target(value: &str) -> Result<String, String> {
    let (rest, default_port) = if let Some(rest) = value.strip_prefix("http://") {
        (rest, 80)
    } else if let Some(rest) = value.strip_prefix("https://") {
        (rest, 443)
    } else {
        return Err("需要以 http:// 或 https:// 开头".to_string());
    };
    let authority = rest.split('/').next().unwrap_or_default().trim();
    if authority.is_empty() {
        return Err("缺少服务器地址".to_string());
    }
    Ok(socket_target(authority, default_port))
}

async fn connect_test_stream(target: String, test_with_proxy: bool) -> Result<Stream, String> {
    if test_with_proxy {
        return hbb_common::socket_client::connect_tcp(target, 1_800)
            .await
            .map_err(|err| format!("连接失败: {err}"));
    }
    match hbb_common::tokio::time::timeout(
        Duration::from_millis(1_800),
        hbb_common::tokio::net::TcpStream::connect(target),
    )
    .await
    {
        Ok(Ok(stream)) => {
            let peer_addr = stream
                .peer_addr()
                .map_err(|err| format!("无法读取服务器地址: {err}"))?;
            Ok(Stream::from(stream, peer_addr))
        }
        Ok(Err(err)) => Err(format!("直连失败: {err}")),
        Err(_) => Err("直连在 1.8 秒内未完成".to_string()),
    }
}

async fn test_id_server_response(target: Option<String>, test_with_proxy: bool) -> String {
    let Some(target) = target else {
        return String::new();
    };
    let mut stream = match connect_test_stream(target, test_with_proxy).await {
        Ok(stream) => stream,
        Err(message) => return message,
    };
    let mut request = RendezvousMessage::new();
    request.set_test_nat_request(TestNatRequest {
        serial: 0,
        ..Default::default()
    });
    if let Err(err) = stream.send(&request).await {
        return format!("无法发送 RustDesk ID 探测: {err}");
    }
    for _ in 0..2 {
        let Some(frame) = stream.next_timeout(1_500).await else {
            return "未收到 RustDesk ID 服务响应".to_string();
        };
        let bytes = match frame {
            Ok(bytes) => bytes,
            Err(err) => return format!("读取 RustDesk ID 响应失败: {err}"),
        };
        let response = match RendezvousMessage::parse_from_bytes(&bytes) {
            Ok(response) => response,
            Err(_) => return "服务器返回的不是 RustDesk ID 协议数据".to_string(),
        };
        if matches!(
            response.union,
            Some(rendezvous_message::Union::TestNatResponse(_))
        ) {
            return String::new();
        }
    }
    "服务器未返回 RustDesk ID 探测响应".to_string()
}

async fn test_relay_server_response(
    target: Option<String>,
    key: String,
    test_with_proxy: bool,
) -> String {
    let Some(target) = target else {
        return String::new();
    };
    let mut left = match connect_test_stream(target.clone(), test_with_proxy).await {
        Ok(stream) => stream,
        Err(message) => return message,
    };
    let mut right = match connect_test_stream(target, test_with_proxy).await {
        Ok(stream) => stream,
        Err(message) => return message,
    };
    static RELAY_TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    let sequence = RELAY_TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let uuid = format!(
        "server-config-test-{}-{sequence}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let mut left_request = RendezvousMessage::new();
    left_request.set_request_relay(RequestRelay {
        id: "server-config-test".to_string(),
        uuid: uuid.clone(),
        licence_key: key.clone(),
        ..Default::default()
    });
    let mut right_request = RendezvousMessage::new();
    right_request.set_request_relay(RequestRelay {
        uuid,
        licence_key: key,
        ..Default::default()
    });
    if let Err(err) = left.send(&left_request).await {
        return format!("无法发送 RustDesk Relay 探测: {err}");
    }
    if let Err(err) = right.send(&right_request).await {
        return format!("无法建立第二条 RustDesk Relay 探测连接: {err}");
    }
    hbb_common::tokio::time::sleep(Duration::from_millis(80)).await;
    let marker = format!("rustdesk-relay-test-{sequence}").into_bytes();
    if let Err(err) = left.send_raw(marker.clone()).await {
        return format!("无法发送 RustDesk Relay 回环数据: {err}");
    }
    for _ in 0..2 {
        let Some(frame) = right.next_timeout(1_500).await else {
            return "Relay 服务器未完成 RustDesk 双端转发".to_string();
        };
        match frame {
            Ok(bytes) if bytes.as_ref() == marker.as_slice() => return String::new(),
            Ok(_) => continue,
            Err(err) => return format!("读取 RustDesk Relay 回环数据失败: {err}"),
        }
    }
    "Relay 服务器未返回正确的 RustDesk 转发数据".to_string()
}

async fn test_api_server_response(api_server: Option<String>, test_with_proxy: bool) -> String {
    let Some(api_server) = api_server else {
        return String::new();
    };
    match hbb_common::tokio::task::spawn_blocking(move || {
        test_api_server_response_sync(&api_server, test_with_proxy)
    })
    .await
    {
        Ok(result) => result,
        Err(err) => format!("RustDesk API 探测任务失败: {err}"),
    }
}

fn test_api_server_response_sync(api_server: &str, test_with_proxy: bool) -> String {
    match librustdesk::validate_rustdesk_api_server(api_server, test_with_proxy) {
        Ok(_) => String::new(),
        Err(err) => format!("RustDesk API 校验失败: {err}"),
    }
}

fn server_config_errors_empty(errors: &Value) -> bool {
    errors
        .as_object()
        .map(|errors| {
            errors
                .values()
                .all(|error| error.as_str().unwrap_or_default().is_empty())
        })
        .unwrap_or(false)
}

fn parse_json_list(raw: &str, name: &str) -> Result<Vec<Value>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<Value>>(raw).map_err(|err| format!("Invalid {}: {}", name, err))
}

fn recent_peer_summary(peer: &Value) -> Value {
    json!({
      "id": json_field_string(peer, "id"),
      "username": json_field_string(peer, "username"),
      "hostname": json_field_string(peer, "hostname"),
      "platform": json_field_string(peer, "platform"),
      "alias": json_field_string(peer, "alias"),
      "hasStoredPassword": !json_field_string(peer, "hash").is_empty()
    })
}

fn lan_peer_summary(peer: &Value, recent: &[Value]) -> Value {
    json!({
      "id": json_field_string(peer, "id"),
      "username": json_field_string(peer, "username"),
      "hostname": json_field_string(peer, "hostname"),
      "platform": json_field_string(peer, "platform"),
      "online": peer.get("online").and_then(Value::as_bool).unwrap_or(false),
      "ip": lan_peer_ip(peer, recent),
      "ipMac": json_object_field(peer, "ip_mac")
    })
}

fn lan_peer_ip(peer: &Value, recent: &[Value]) -> String {
    let id = json_field_string(peer, "id");
    if let Some(preferred) = lan_preferred_ip_store().lock().unwrap().get(&id).cloned() {
        return preferred;
    }
    let mut addresses = peer
        .get("ip_mac")
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .keys()
                .filter_map(|value| IpAddr::from_str(value).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    addresses.sort_unstable_by_key(|address| (!address.is_ipv4(), *address));
    let username = json_field_string(peer, "username");
    let hostname = json_field_string(peer, "hostname");
    if let Some(recent_address) = recent.iter().find_map(|row| {
        let candidate = IpAddr::from_str(&json_field_string(row, "id")).ok()?;
        if !addresses.contains(&candidate) {
            return None;
        }
        let recent_username = json_field_string(row, "username");
        let recent_hostname = json_field_string(row, "hostname");
        ((!username.is_empty() && username == recent_username)
            || (!hostname.is_empty() && hostname == recent_hostname))
            .then_some(candidate)
    }) {
        return recent_address.to_string();
    }
    // Multi-homed peers can answer the same discovery scan from multiple LAN
    // addresses. Prefer the highest IPv4 candidate when no previously working
    // recent-session address identifies the route; this avoids deterministically
    // selecting the oldest/lowest DHCP address from the map.
    addresses
        .iter()
        .rev()
        .find(|address| address.is_ipv4())
        .or_else(|| addresses.first())
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn address_book_peer_summary(peer: &Value) -> Value {
    json!({
      "id": json_field_string(peer, "id"),
      "username": json_field_string(peer, "username"),
      "hostname": json_field_string(peer, "hostname"),
      "platform": json_field_string(peer, "platform"),
      "alias": json_field_string(peer, "alias"),
      "tags": json_array_field(peer, "tags")
    })
}

fn normalize_target(peer_target: &str, force_relay: bool) -> NormalizedPeerTarget {
    let trimmed = peer_target.trim();
    let (id_part, server_part) = match trimmed.split_once('@') {
        Some((id, server)) => (id, Some(server)),
        None => (trimmed, None),
    };
    let (normalized_id, relay_suffix_requested) = strip_relay_suffix(id_part);

    let (custom_server, server_key) = match server_part {
        Some(server_and_query) => match server_and_query.split_once('?') {
            Some((server, query)) => (non_empty_string(server), query_parameter(query, "key")),
            None => (non_empty_string(server_and_query), None),
        },
        None => (None, None),
    };

    NormalizedPeerTarget {
        peer_target: trimmed.to_string(),
        normalized_peer_id: normalized_id.to_string(),
        custom_server,
        server_key,
        relay_suffix_requested,
        effective_force_relay: force_relay || relay_suffix_requested,
    }
}

fn strip_relay_suffix(id: &str) -> (&str, bool) {
    if let Some(stripped) = id.strip_suffix("/r") {
        return (stripped, true);
    }
    if let Some(stripped) = id.strip_suffix(r"\r") {
        return (stripped, true);
    }
    (id, false)
}

fn query_parameter(query: &str, wanted_key: &str) -> Option<String> {
    query.split('&').find_map(|item| {
        let (key, value) = item.split_once('=')?;
        if key.eq_ignore_ascii_case(wanted_key) && !value.trim().is_empty() {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn upstream_status_value() -> Value {
    let blockers = compile_blockers();

    json!({
      "rustdeskRepo": RUSTDESK_UPSTREAM_REPO,
      "rustdeskPath": BUILD_RUSTDESK_PATH,
      "rustdeskSnapshotPresent": build_rustdesk_snapshot_present(),
      "hbbCommonPath": BUILD_HBB_COMMON_PATH,
      "hbbCommonPresent": build_hbb_common_present(),
      "realSessionBindingImplemented": REAL_SESSION_BINDING_IMPLEMENTED,
      "coreBindingReady": core_binding_ready(),
      "blocker": core_binding_blocker(),
      "compileBlockers": blockers
    })
}

fn core_binding_ready() -> bool {
    build_rustdesk_snapshot_present()
        && build_hbb_common_present()
        && REAL_SESSION_BINDING_IMPLEMENTED
}

fn core_binding_blocker() -> Option<String> {
    if !build_rustdesk_snapshot_present() {
        return Some("Missing third_party/rustdesk source snapshot.".to_string());
    }
    if !build_hbb_common_present() {
        return Some(
            "Missing third_party/rustdesk/libs/hbb_common submodule snapshot.".to_string(),
        );
    }
    if !REAL_SESSION_BINDING_IMPLEMENTED {
        return Some(CORE_BLOCKER_MESSAGE.to_string());
    }
    None
}

fn compile_blockers() -> Vec<String> {
    let mut blockers = Vec::new();

    if !build_rustdesk_snapshot_present() {
        blockers.push("Missing third_party/rustdesk source snapshot.".to_string());
    }
    if !build_hbb_common_present() {
        blockers
            .push("Missing third_party/rustdesk/libs/hbb_common submodule snapshot.".to_string());
    }

    blockers.push(
        "Headless RustDesk sessions still rely on Harmony-specific product integration for session orchestration and event handling.".to_string(),
    );
    blockers.push(
        "Some OHOS paths still use intentional stubs for clipboard, connection-manager UI, and audio helpers that are out of scope for the current client-only port.".to_string(),
    );
    blockers.push(
        "Session start remains headless; the remaining work is product integration on the ArkTS side, not RustDesk core call-through.".to_string(),
    );

    blockers
}

fn build_rustdesk_snapshot_present() -> bool {
    BUILD_RUSTDESK_SNAPSHOT_PRESENT == "true"
}

fn build_hbb_common_present() -> bool {
    BUILD_HBB_COMMON_PRESENT == "true"
}

fn query_decoder_capability(label: &str, mime: &[u8], bundled_software_name: &str) -> Value {
    let mime = mime.as_ptr().cast::<c_char>();
    let mime_string = unsafe { CStr::from_ptr(mime).to_string_lossy().into_owned() };
    let recommended = unsafe { OH_AVCodec_GetCapability(mime, false) };
    let hardware =
        unsafe { OH_AVCodec_GetCapabilityByCategory(mime, false, OH_AVCodecCategory::Hardware) };
    let software =
        unsafe { OH_AVCodec_GetCapabilityByCategory(mime, false, OH_AVCodecCategory::Software) };
    let platform_software_name = capability_name(software);
    let software_name = if platform_software_name.is_empty() {
        bundled_software_name.to_string()
    } else {
        platform_software_name
    };
    log::info!(
        "OHOS capability query codec={} mime={} recommended={:p} hardware={:p} software={:p}",
        label,
        mime_string,
        recommended,
        hardware,
        software
    );

    json!({
      "codec": label,
      "mime": mime_string,
      "recommendedAvailable": !recommended.is_null(),
      "recommendedName": capability_name(recommended),
      "recommendedIsHardware": capability_is_hardware(recommended),
      "hardwareAvailable": !hardware.is_null(),
      "hardwareName": capability_name(hardware),
      "softwareAvailable": !software.is_null() || !bundled_software_name.is_empty(),
      "softwareName": software_name
    })
}

fn capability_name(capability: *mut OH_AVCapability) -> String {
    if capability.is_null() {
        return String::new();
    }
    let name = unsafe { OH_AVCapability_GetName(capability) };
    if name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
    }
}

fn capability_is_hardware(capability: *mut OH_AVCapability) -> bool {
    if capability.is_null() {
        false
    } else {
        unsafe { OH_AVCapability_IsHardware(capability) }
    }
}
