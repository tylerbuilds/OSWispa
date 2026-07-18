fn main() {
    if std::env::var_os("CARGO_FEATURE_DESKTOP_RUNTIME").is_some() {
        tauri_build::build();
    } else {
        println!("cargo:rerun-if-changed=tauri.conf.json");
    }
}
