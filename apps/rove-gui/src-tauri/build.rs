fn main() {
    println!("cargo:rerun-if-changed=../dist");
    if std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some() {
        let index = std::fs::read_to_string("../dist/index.html")
            .expect("Packaged GUI requires dist/index.html; run npm run build first");
        assert!(
            index.contains("id=\"app\""),
            "Invalid packaged GUI index.html"
        );
        // Vite clears dist while rebuilding. Never build the native GUI in
        // parallel with a frontend build (including Android beforeBuildCommand).
        for part in index.split("\"/assets/").skip(1) {
            let asset = part.split('"').next().unwrap();
            assert!(
                std::path::Path::new("../dist/assets").join(asset).is_file(),
                "Missing packaged GUI asset: {asset}"
            );
        }
    }
    tauri_build::build()
}
