// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Elevated helper mode (MFT scans) runs and exits without any UI.
    if let Some(code) = omnihub_core::helper::run_if_helper(&args) {
        std::process::exit(code);
    }
    // Started by Brave/Chrome/Edge for the OmniHub extension: relay to the
    // running app and exit (no window).
    if let Some(origin) = omnihub_core::browser::host::invoked_as_host(&args) {
        omnihub_core::browser::host::run(origin);
        return;
    }
    omnihub_lib::run(args);
}
