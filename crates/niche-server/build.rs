//! Ensures `web/dist` exists so the crate compiles before the frontend is built. The placeholder
//! page tells the user how to build the UI; release builds run `npm run build` first
//! (scripts/build-release.sh).

fn main() {
    let dist = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/dist");
    println!("cargo:rerun-if-changed=../../web/dist");
    if !dist.join("index.html").exists() {
        let _ = std::fs::create_dir_all(&dist);
        let _ = std::fs::write(
            dist.join("index.html"),
            "<!doctype html><meta charset=utf-8><title>NicheRS</title><body style=\"font-family:system-ui;margin:3rem\"><h1>NicheRS API is running</h1><p>The web interface was not built into this binary. Run <code>npm --prefix web ci &amp;&amp; npm --prefix web run build</code> and rebuild, or use the API at <code>/api/v1</code>.</p>",
        );
    }
}
