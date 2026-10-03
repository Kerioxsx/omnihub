// The companion server embeds the phone web app from ../../dist-mobile
// (built with `npm run build`). Make sure the folder exists so the crate
// also builds before the frontend has been built.
fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist-mobile");
    if !dir.join("index.html").exists() {
        std::fs::create_dir_all(&dir).expect("create dist-mobile");
        std::fs::write(
            dir.join("index.html"),
            "<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><title>OmniHub</title><body style='font-family:system-ui;background:#0b0b12;color:#eee;display:grid;place-items:center;height:100vh'><p>The phone app was not built. Run <code>npm run build</code> in omnihub/ and rebuild.</p>",
        )
        .expect("write placeholder");
    }
    println!("cargo:rerun-if-changed=../../dist-mobile");
}
