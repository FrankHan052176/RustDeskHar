use librustdesk::{
    flutter, flutter_ffi,
    platform::ohos::{self, DirectRenderTarget},
};
use napi_derive_ohos::napi;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::{c_char, c_void, CStr, CString},
    fs,
    net::IpAddr,
    slice,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};

#[repr(C)]
struct OH_AVCapability {
    _private: [u8; 0],
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

const MAX_EVENTS_PER_SESSION: usize = 512;
const MAX_INPUT_EVENTS: usize = 256;
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
    let name = event.get("name").and_then(Value::as_str).unwrap_or_default();
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

unsafe extern "C" {
    fn session_get_rgba(session_uuid_str: *const c_char, display: usize) -> *const u8;
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
        return background_job_failure(
            "runtime_poll_account_oidc",
            "provider",
            failed_message,
        )
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
    poll_background_job(
        address_book_job_store(),
        "runtime_poll_address_book_sync",
    )
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
pub fn runtime_test_server_config(config_json: String) -> String {
    let config = match parse_server_config(&config_json) {
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
    let errors = test_server_config(&config);
    let ok = server_config_errors_empty(&errors);
    json!({
      "ok": ok,
      "action": "runtime_test_server_config",
      "message": if ok { "Server configuration is valid" } else { "Invalid server configuration" },
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
    let errors = test_server_config(&config);
    if !server_config_errors_empty(&errors) {
        return json!({
          "ok": false,
          "action": "runtime_set_server_config",
          "message": "Invalid server configuration",
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

    json!({
      "ok": true,
      "action": "runtime_set_server_config",
      "message": "Server configuration saved",
      "config": server_config_snapshot(),
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
            let peers = peers
                .iter()
                .map(recent_peer_summary)
                .collect::<Vec<_>>();
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
    flutter_ffi::main_discover();
    json!({
      "ok": true,
      "action": "runtime_scan_lan_peers",
      "message": "LAN discovery started",
      "pollAfterMs": 300,
      "timeoutMs": 4000
    })
    .to_string()
}

#[napi]
pub fn runtime_list_lan_peers() -> String {
    let raw = serde_json::to_string(&hbb_common::config::LanPeers::load().peers)
        .unwrap_or_default();
    match parse_json_list(&raw, "LAN peer list") {
        Ok(peers) => {
            let peers = peers.iter().map(lan_peer_summary).collect::<Vec<_>>();
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
        && json_bool(&options, &["isViewOnly", "is_view_only", "viewOnly", "view_only"]);

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
        flutter_ffi::session_toggle_option(
            core_session_id.clone(),
            VIEW_ONLY_OPTION.to_string(),
        );
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
            return (false, "Input is disabled for a view-only session".to_string());
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
            return (false, "Input is disabled for a view-only session".to_string());
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
                format!("Prepared RustDesk codec preference for first login: {}", normalized)
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
            return (false, "Input is disabled for a view-only session".to_string());
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
            return (false, "Input is disabled for a view-only session".to_string());
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
            return (false, "Keyboard capture is disabled for a view-only session".to_string());
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
            return (false, "Input is disabled for a view-only session".to_string());
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
            return (false, "Chat is disabled for a view-only session".to_string());
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
            return (false, "Notes are disabled for a view-only session".to_string());
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
            return (false, "File access is disabled for a view-only session".to_string());
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
            return (false, "File transfer is disabled for a view-only session".to_string());
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
                return (false, "File transfer is disabled for a view-only session".to_string());
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
    let size = flutter_ffi::session_get_rgba_size(core_session_id, display as usize).0;
    if size == 0 {
        return Vec::new().into();
    }
    let Ok(core_session_id) = CString::new(core_session_id.to_string()) else {
        return Vec::new().into();
    };
    let frame = unsafe { session_get_rgba(core_session_id.as_ptr(), display as usize) };
    if frame.is_null() {
        return Vec::new().into();
    }
    let copied = unsafe { slice::from_raw_parts(frame, size).to_vec() };
    copied.into()
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
                background_job_failure(action, "internal", "Background operation failed".to_string())
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
    let raw = librustdesk::common::http_request_sync(
        url,
        method.to_string(),
        body,
        headers,
    )
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
    Ok(ApiResponse {
        status_code,
        body,
    })
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
                && flutter_ffi::main_get_local_option("access_token".to_string()).0
                    == access_token
            {
                clear_account_state();
            }
        }
    }
    require_api_success(response, context)
}

fn login_request_body(username: &str, password: Option<&str>) -> Value {
    let device_info_raw = flutter_ffi::main_get_login_device_info().0;
    let device_info = serde_json::from_str::<Value>(&device_info_raw)
        .unwrap_or_else(|_| json!({}));
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
    let device_info = serde_json::from_str::<Value>(&device_info_raw)
        .unwrap_or_else(|_| json!({}));
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
                Err(message) => background_job_failure(
                    "runtime_poll_account_action",
                    "transport",
                    message,
                ),
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
                Ok(response) => {
                    handle_account_response(
                        response,
                        &challenge.api_server,
                        &challenge.username,
                        generation,
                    )
                }
                Err(message) => background_job_failure(
                    "runtime_poll_account_action",
                    "transport",
                    message,
                ),
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
    result.insert(
        "alias".to_string(),
        json_field_string(peer, "alias").into(),
    );
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
    let data_raw = body
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default();
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
        let path = format!(
            "/api/ab/shared/profiles?current={}&pageSize=100",
            current
        );
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
        let path = format!(
            "/api/ab/peers?current={}&pageSize=100&ab={}",
            current, guid
        );
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
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "transport",
                message,
            );
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
        match api_request(
            &api_server,
            "/api/ab",
            "GET",
            None,
            Some(&access_token),
        )
        .and_then(|response| {
            legacy_address_book_entry(response, &api_server, &access_token, generation)
        })
        {
            Ok(entry) => vec![entry],
            Err(message) => {
                return background_job_failure(
                    "runtime_poll_address_book_sync",
                    "server",
                    message,
                );
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
                return background_job_failure(
                    "runtime_poll_address_book_sync",
                    "server",
                    message,
                );
            }
        };
        let personal_guid = json_field_string(&personal_body, "guid")
            .trim()
            .to_string();
        if personal_guid.is_empty() {
            return background_job_failure(
                "runtime_poll_address_book_sync",
                "protocol",
                "Personal address book has no guid".to_string(),
            );
        }
        let mut profiles = vec![(
            personal_guid.clone(),
            "My address book".to_string(),
            true,
        )];
        match fetch_shared_address_book_profiles(&api_server, &access_token, generation) {
            Ok(shared) => {
                for (guid, name) in shared {
                    if guid != personal_guid {
                        profiles.push((guid, name, false));
                    }
                }
            }
            Err(message) => {
                return background_job_failure(
                    "runtime_poll_address_book_sync",
                    "server",
                    message,
                );
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
            let (tags, tag_colors) = match fetch_address_book_tags(
                &api_server,
                &access_token,
                &guid,
                generation,
            ) {
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
        json_raw_string(
            &payload,
            &["idServer", "customRendezvousServer", "host"],
        )
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

fn test_server_config(config: &Value) -> Value {
    let id_server = json_field_string(config, "idServer");
    let relay_server = json_field_string(config, "relayServer");
    let api_server = json_field_string(config, "apiServer");
    let test_with_proxy = config
        .get("testWithProxy")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let id_error = if id_server.is_empty() {
        "required".to_string()
    } else {
        flutter_ffi::main_test_if_valid_server(id_server, test_with_proxy)
    };
    let relay_error = if relay_server.is_empty() {
        String::new()
    } else {
        flutter_ffi::main_test_if_valid_server(relay_server, test_with_proxy)
    };
    let api_error = if api_server.is_empty()
        || api_server.starts_with("http://")
        || api_server.starts_with("https://")
    {
        String::new()
    } else {
        "invalid_http".to_string()
    };
    json!({
      "idServer": id_error,
      "relayServer": relay_error,
      "apiServer": api_error
    })
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

fn lan_peer_summary(peer: &Value) -> Value {
    json!({
      "id": json_field_string(peer, "id"),
      "username": json_field_string(peer, "username"),
      "hostname": json_field_string(peer, "hostname"),
      "platform": json_field_string(peer, "platform"),
      "online": peer.get("online").and_then(Value::as_bool).unwrap_or(false),
      "ip": lan_peer_ip(peer),
      "ipMac": json_object_field(peer, "ip_mac")
    })
}

fn lan_peer_ip(peer: &Value) -> String {
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
    addresses
        .first()
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
