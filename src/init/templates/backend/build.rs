fn main() {
    // Use the environment variable, not cfg!(), for feature detection in build.rs
    if std::env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        let pkg = std::path::Path::new("../frontend/pkg");
        if !pkg.exists() || pkg.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            panic!(
                "\n\nembedding assets requires frontend/pkg/ to exist and be non-empty.\n\
                 Run this first:\n\n  \
                 wasm-drydock release\n\n"
            );
        }

        let css = std::path::Path::new("../frontend/styles/screen.css");
        if !css.exists() {
            panic!(
                "\n\nembedding assets requires frontend/styles/screen.css to exist.\n\
                 Run this first:\n\n  \
                 wasm-drydock release\n\n"
            );
        }

        let public = std::path::Path::new("../frontend/public");
        if !public.exists() {
            panic!(
                "\n\nembedding assets requires frontend/public/ to exist.\n\
                 Create it first, even if empty.\n\n"
            );
        }
    }
}
