//! omnihub-browser-host: the stand-alone native-messaging host (the
//! OmniHub app is its own host; this one serves the headless build and
//! development). See `omnihub_core::browser::host`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    omnihub_core::browser::host::run(omnihub_core::browser::host::invoked_as_host(&args).unwrap_or_default());
}
