// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Elevated helper mode (MFT scans) runs and exits without any UI.
    if let Some(code) = omnihub_core::helper::run_if_helper(&args) {
        std::process::exit(code);
    }
    omnihub_lib::run(args);
}
