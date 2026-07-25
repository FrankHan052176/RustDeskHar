use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    let build_stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // Keep the runtime capability metadata aligned with the actual Cargo path
    // dependency used by this workspace. The previous ../Core probe made a
    // fully linked build report that its RustDesk source snapshot was absent.
    let rustdesk_root = manifest_dir.join("../R_RustDesk-Core");
    let hbb_common_dir = rustdesk_root.join("libs/hbb_common");

    println!("cargo:rerun-if-changed={}", rustdesk_root.display());
    println!("cargo:rerun-if-changed={}", hbb_common_dir.display());
    println!(
        "cargo:rustc-env=BUILD_RUSTDESK_SNAPSHOT_PRESENT={}",
        rustdesk_root.exists()
    );
    println!(
        "cargo:rustc-env=BUILD_HBB_COMMON_PRESENT={}",
        hbb_common_dir.exists()
    );
    println!(
        "cargo:rustc-env=BUILD_RUSTDESK_PATH={}",
        rustdesk_root.display()
    );
    println!(
        "cargo:rustc-env=BUILD_HBB_COMMON_PATH={}",
        hbb_common_dir.display()
    );
    println!(
        "cargo:rustc-env=BUILD_MARKER=har-build-{}-{}",
        build_stamp,
        std::process::id()
    );

    napi_build_ohos::setup();
}
